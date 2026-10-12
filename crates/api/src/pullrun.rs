//! ONE PRESS, AND THE SERVER OWNS EVERYTHING AFTER IT.
//!
//! # What this replaces, and why it had to move
//!
//! The multi-pass loop used to live in the browser. `web/src/routes/ingest`
//! fired one request per leg, re-read the census, and decided whether to fire
//! the whole set again -- up to four hundred times. It was correct about what
//! to retry and it put the operator's backfill inside a browser tab.
//!
//! The symptom was a stop that looked like a restart. A leg that answered 502
//! ended the pass; the page waited twenty seconds and fired the set again; the
//! operator watched `Pull running` become `NOT STARTED` and then, unasked, a
//! fresh run. Reported on 2026-08-20 in those words: *"why stopped and again
//! auto repulled ... only one pull from the webpage and automatically
//! everything should be entirely taken care internally"*.
//!
//! # Why it is not one long request either
//!
//! The obvious alternative -- hold the connection open until the window is
//! satisfied -- is the failure this repository has already had. A socket held
//! across a whole backfill is a socket that drops, and the drop is recorded as
//! `HTTP 0`, which is what abandoned Dhan's options. So the run is neither in
//! the tab nor on the wire: **it is a task on this process**, started by a
//! request that returns at once and observed through a status document the page
//! polls. Nothing is held open, and nothing is driven from outside.
//!
//! # The two shapes the operator asked for, and where each one is
//!
//! | Rule | Where it is enforced |
//! |---|---|
//! | Feeds run in PARALLEL | [`conduct`] spawns one task per vendor |
//! | Inside one feed, legs run SEQUENTIALLY | each task awaits its legs in turn |
//!
//! It is not a preference on either side. A rate budget is per VENDOR, so two
//! legs fired at one broker together spend one ceiling twice, while two
//! different brokers share no ceiling, no census lock and no manifest at all.
//!
//! # Cost
//!
//! O(1) per leg beyond the leg's own work: the group lookup is a linear scan of
//! the vendor list, which is bounded by the number of feeds this build has
//! (`pull::vendor::FEED_COUNT`), not by the number of legs. That bound holds
//! because [`legs_from`] refuses a leg naming no feed (D-0906), which
//! `a_leg_naming_no_feed_refuses_the_run_so_groups_never_outnumber_feeds`
//! pins. The pass loop holds one `Progress` and one checkpoint per leg;
//! nothing accumulates per pass.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use crate::census;
use crate::server::{Loaded, Site, percent_decode};

/// How many legs one press may carry: one per feed, per rung the ingest page
/// offers (`1s`, `1min`, `1day`), per route (`/pull/spot`, `/pull/fno`).
///
/// A form with more is refused by name ([`Refusal::TooManyLegs`]) rather than
/// read, so [`MAX_RUN_FORM_BYTES`] has a count to be sized from (P3-01-01,
/// D-1769).
pub const MAX_RUN_LEGS: usize = pull::vendor::FEED_COUNT * 3 * 2;

/// The worst size of one `leg=` field: a member form the inner route admits
/// ([`crate::ingest::MAX_MEMBER_FORM_BYTES`]) plus an ordinary form's worth of
/// envelope, percent-encoded twice more by the page. Encoding an
/// already-encoded byte turns `%` into `%25`, so each pass costs at most a
/// further two bytes per original escape: five bytes per form byte in all.
pub const MAX_LEG_FIELD_BYTES: usize =
    "leg=".len() + 5 * (crate::ingest::MAX_MEMBER_FORM_BYTES + crate::server::MAX_FORM_BYTES);

/// The body `/pull/run` and `/pull/recovery` read.
///
/// Both carry legs, and each leg repeats its member list twice-encoded, so the
/// shared 8 KiB bound answered a framework 413 in plain text at about 340
/// ticked members on one leg and about 55 across six; the page then reported
/// a `SyntaxError` instead of a reason (P3-01-01, D-1769). Sized so every run
/// [`legs_from`] would accept is read, and one leg too many reaches
/// [`Refusal::TooManyLegs`]. 27,347,836 bytes at most — about 27.3 MB, or
/// 26.1 MiB — held in memory once; see `docs/06-limits.md`. (This said "about
/// 26.5 MB", which is neither unit; tests-docs-security-pass17 P17-19,
/// D-1967, and `the_run_form_bound_is_the_figure_the_limits_document_states`.)
pub const MAX_RUN_FORM_BYTES: usize =
    crate::server::MAX_FORM_BYTES + (MAX_RUN_LEGS + 1) * MAX_LEG_FIELD_BYTES;

/// How many passes one press may make.
///
/// A window can be larger than one sitting at a legal rate, so the run keeps
/// going. This is not a guess at how many passes are needed -- it is a stop for
/// a loop that would otherwise be unbounded if the vendor never recovered, and
/// it is REPORTED when it is reached rather than being silently the end.
pub const MAX_PASSES: u32 = 400;

/// How many passes without retryable failures or store growth end idle retries.
///
/// Empty receipts and already-present bars can both produce this outcome. It
/// does not prove full basket coverage. Permanent refusals halt their feed;
/// transient failures retry up to [`MAX_PASSES`].
pub const CLEAN_EMPTY_PASSES: u32 = 3;

/// How often the store is re-counted while a pass is still running.
///
/// Five seconds. One manifest read per vendor, against a page that polls the
/// document it feeds every two — so this is the cheaper half of the pair, and
/// it is the half that makes the other one worth reading.
pub const ROWS_TICK: core::time::Duration = core::time::Duration::from_secs(5);

/// The gap between a pass that FAILED and the next one.
///
/// Without it a dropped socket becomes a spin: the loop re-issues instantly,
/// fails instantly, and burns the vendor's goodwill while making no progress.
/// The governor backs off INSIDE a pass; this is the gap between passes, which
/// nothing else covers.
pub const RETRY_WAIT: core::time::Duration = core::time::Duration::from_secs(20);

/// Which route a leg is bound for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// `POST /pull/spot`.
    Spot,
    /// `POST /pull/fno`.
    Fno,
}

impl Route {
    /// The path this route is served on, for the status document.
    #[must_use]
    pub fn path(self) -> &'static str {
        match self {
            Self::Spot => "/pull/spot",
            Self::Fno => "/pull/fno",
        }
    }

    /// Reads a route from the wire, refusing anything that is not one of the
    /// two. A third name is a WIRING FAULT in the page and is named rather than
    /// mapped onto a default -- a leg silently routed to spot would pull the
    /// wrong thing and report success.
    fn parse(word: &str) -> Option<Self> {
        match word {
            "/pull/spot" => Some(Self::Spot),
            "/pull/fno" => Some(Self::Fno),
            _ => None,
        }
    }
}

/// One leg: a single form body bound for a single route on a single vendor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leg {
    /// Where it is sent.
    pub route: Route,
    /// Whose ceiling it spends, and therefore which chain it belongs to.
    pub vendor: String,
    /// The rung, for the ladder sort. `1day`, `1min`, `futures`, `options`.
    pub dir: String,
    /// What to call it on the status document.
    pub label: String,
    /// The form body, verbatim, exactly as the old per-leg routes take it.
    pub body: String,
}

/// Where a rung sits on the ladder `pull::fold` requires.
///
/// The day pass for a month must land before the minute pass for it, and spot
/// must land before the derivatives that reference it. A rung nobody ranked
/// goes LAST, never first: `position` answers `None` for an unknown rung, and
/// sorting those first is the one position that breaks the ladder.
#[must_use]
pub fn ladder_rank(dir: &str) -> usize {
    const PULL_ORDER: [&str; 5] = ["1day", "1min", "1s", "futures", "options"];
    PULL_ORDER
        .iter()
        .position(|known| *known == dir)
        .unwrap_or(PULL_ORDER.len())
}

/// The label of the receipt fact that says what another attempt at a leg
/// could change. The server writes it where each fault is still typed (an
/// instrument that did not answer, a member that answered with gaps, a store
/// that will not take a write) and [`leg_outcome`] reads the word back, so
/// the conductor never classifies a failure from its prose (DB-B26, DB-B35).
pub const RETRY_CLASS_FACT: &str = "Retry class";

/// What another attempt at a leg could change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryClass {
    /// Something did not answer: asking again can succeed.
    Transient,
    /// Every failure is from an instrument that answered, or is a refusal no
    /// later pass of this run can lift: asking again returns the same.
    Standing,
    /// The store will not take a write: nothing asked can land.
    Store,
}

impl RetryClass {
    /// The word on the receipt, never renamed.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Standing => "standing",
            Self::Store => "store_unavailable",
        }
    }

    /// The class a receipt states, or `None` when it carries no such fact
    /// (a door refusal, or a receipt from a route that does not class).
    #[must_use]
    pub fn of_receipt(html: &str) -> Option<Self> {
        let said = receipt_fact(html, RETRY_CLASS_FACT)?;
        let word = said.split_whitespace().next()?;
        [Self::Transient, Self::Standing, Self::Store]
            .into_iter()
            .find(|class| class.word() == word)
    }
}

/// Why a run ended, as a code a page can match and a sentence cannot blur
/// (DB-B26): `/pull/run.json` carries it as `stopReason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// [`CLEAN_EMPTY_PASSES`] passes reported nothing owed and no growth.
    Idle,
    /// As `Idle`, with legs that finished with standing failures.
    Standing,
    /// [`MAX_PASSES`] passes ran: the runaway stop.
    Ceiling,
    /// The operator pressed stop.
    Operator,
    /// A feed was halted: a dead credential or a fixed refusal.
    Halted,
    /// The store would not take a write (DB-B35).
    StoreUnavailable,
}

impl Stop {
    /// The code, never renamed.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Standing => "standing_failures",
            Self::Ceiling => "pass_ceiling",
            Self::Operator => "stopped",
            Self::Halted => "halted",
            Self::StoreUnavailable => "store_unavailable",
        }
    }
}

/// What one vendor's chain is doing, for the status document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedReport {
    /// Whose chain this is.
    pub vendor: String,
    /// How many legs it carries.
    pub legs: u32,
    /// How many attempted legs returned this pass, including failed responses.
    /// Skipped legs and an interrupted in-flight leg are not counted.
    pub legs_done: u32,
    /// The label of the leg on the wire right now.
    pub doing: String,
    /// How many subsequent passes actually retried this feed after a failure.
    pub retries: u32,
    /// The first transient failure, replaced by a terminal cause if one occurs.
    pub last_error: Option<String>,
    /// Whether this feed's latest pass ended, including a refusal or stop.
    pub finished: bool,
    /// Whether this feed's CREDENTIAL is dead, so re-asking cannot help.
    ///
    /// # Why this is a field and not another `last_error`
    ///
    /// Because it changes what the loop DOES, not only what the page says.
    /// A dead token halts its feed, as does a fixed request/preflight refusal.
    /// Transient failures such as dropped sockets and throttles remain eligible
    /// for retry up to the pass ceiling. §8 forbids minting a token here, so a
    /// credential halt requires access to be corrected outside this run.
    ///
    /// Set once, never cleared for the life of the run. It is per FEED, because
    /// a credential is — the same reason the seats and the governors are.
    pub credential_dead: bool,
    /// Legs that finished with standing failures: the vendor answered and
    /// what it sent has gaps, or a refusal no later pass can lift. Each is
    /// settled for the run and not asked again (DB-B26).
    pub standing: u32,
    /// The store would not take this feed's writes, so the feed is halted
    /// (DB-B35).
    pub store_unavailable: bool,
    /// Legs not attempted this pass because they were already clean in this
    /// retry cycle, or because of a dependency, terminal refusal or stop. An
    /// interrupted in-flight request is neither done nor skipped.
    ///
    /// **A request not made leaves no trace, which is exactly why it needs a
    /// counter.** An expired derivative is priced against the underlying's bar
    /// at the same minute, so a feed whose spot leg failed cannot price one —
    /// and every derivative request behind it would be spent against a shared
    /// token to be refused. Skipping them is right; skipping them *silently*
    /// would leave the page showing a feed that did less work for no stated
    /// reason, which is the `CLAUDE.md` §4 failure wearing a success's clothes.
    ///
    /// Separate from `last_error`, which deliberately keeps the FIRST cause: the
    /// spot failure is what an operator must read, and the skips are its
    /// consequence. One number says how much was deferred; the error says why.
    pub skipped: u32,
}

/// The whole run, as the page reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// How many passes have completed.
    pub passes: u32,
    /// How many subsequent passes actually retried at least one failed feed.
    pub retries: u32,
    /// The store's row count when the run started; `None` when the census
    /// total does not fit a `u64` (see [`rows_now`]).
    pub rows_at_start: Option<u64>,
    /// The store's row count as of the last pass; `None` as above.
    pub rows_now: Option<u64>,
    /// One per vendor, in the order the feeds were ticked.
    pub feeds: Vec<FeedReport>,
    /// The summary, once there is one. `None` means still running, and it is
    /// the ONLY reading of "still running" -- see `Finisher` for why a panic
    /// cannot leave it `None` forever.
    pub finished: Option<String>,
    /// Whether the operator has asked it to stop at the next leg.
    pub stopping: bool,
    /// Whether this document describes a run that was ever STARTED.
    ///
    /// # The bug this field exists to fix, caught by its own test
    ///
    /// `running` was `finished.is_none()` alone. `Progress::default()` has no
    /// summary, so a site that had never run answered `"running":true` — and
    /// the page asks `/pull/run.json` on load. Every fresh tab would have
    /// believed a backfill was in flight, watched a run that did not exist, and
    /// never shown the Pull button again.
    ///
    /// `Default` must stay the never-ran document, because that is exactly what
    /// the route renders for an empty slot. So the distinction lives here
    /// rather than in a second constructor nobody is obliged to call.
    pub started: bool,
    /// Which claim this document belongs to. `0` is the never-claimed
    /// `Default`; every [`Progress::claimed`] takes the next number from one
    /// process-wide counter, so two claims never share one.
    ///
    /// # Why a run carries a number and not just the slot
    ///
    /// The slot outlives the run that filled it. A run publishes its summary,
    /// which frees the slot, and only THEN drops its [`Finisher`] and its
    /// last ticker poll. A second press admitted in that gap installed a fresh
    /// document, and the first run's `Finisher` saw that document's
    /// `finished: None` and wrote "ended abnormally" onto a live run, which
    /// freed the slot for a third press over the same store. Every edit a run
    /// makes is now addressed to its own number and lands nowhere once the slot
    /// holds another. Not rendered: the page has no use for it. D-2760.
    pub generation: u64,
    /// When the run was claimed, in Unix milliseconds by the server's clock,
    /// or `None` for the never-ran document.
    ///
    /// # Why (DB-B32)
    ///
    /// A tab reloaded mid-run, or a second tab, restarted the elapsed clock at
    /// its own load and measured the run against its own feed and selection,
    /// because this document carried no start time and no account of what was
    /// asked: a Zerodha run watched with GDFL active read "Reading Global
    /// Datafeeds", with GDFL's census as its denominator. The start and the
    /// legs below are the run's own, so a resumed page adopts them read-only.
    pub started_at_ms: Option<u64>,
    /// What each leg asked for, in the order the run was sent (DB-B32).
    pub asked: Vec<AskedLeg>,
    /// Why the run ended, once it has (DB-B26).
    pub stop: Option<Stop>,
}

/// One leg as it was asked: its feed, rung, label and the window and
/// instruments its form names. Read from the leg's own form body with the same
/// [`crate::server::param`] the route that runs it uses, never re-derived.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AskedLeg {
    /// The route it is sent to.
    pub route: String,
    /// The feed's wire key.
    pub vendor: String,
    /// The rung (`1day`, `1min`, `futures`, `options`).
    pub dir: String,
    /// The page's label for it.
    pub label: String,
    /// The form's `target` (the universe it names), or empty.
    pub target: String,
    /// The form's first day, as sent.
    pub from: String,
    /// The form's last day, as sent.
    pub to: String,
    /// How many `member=` fields the form carries; zero means every member of
    /// the target, which is how the page sends "all ticked".
    pub members: usize,
}

impl AskedLeg {
    /// The leg as its own form names it.
    #[must_use]
    pub fn of(leg: &Leg) -> Self {
        let field = |name: &str| crate::server::param(&leg.body, name);
        Self {
            route: leg.route.path().to_owned(),
            vendor: leg.vendor.clone(),
            dir: leg.dir.clone(),
            label: leg.label.clone(),
            target: field("target"),
            from: field("from"),
            to: field("to"),
            members: crate::server::params(&leg.body, "member").len(),
        }
    }

    /// The leg as one JSON object, appended to `out`.
    fn json(&self, out: &mut String) {
        for (nth, (name, value)) in [
            ("route", &self.route),
            ("vendor", &self.vendor),
            ("dir", &self.dir),
            ("label", &self.label),
            ("target", &self.target),
            ("from", &self.from),
            ("to", &self.to),
        ]
        .into_iter()
        .enumerate()
        {
            out.push(if nth == 0 { '{' } else { ',' });
            out.push('"');
            out.push_str(name);
            out.push_str("\":");
            out.push_str(&quote_for_json(value));
        }
        out.push_str(",\"members\":");
        out.push_str(&self.members.to_string());
        out.push('}');
    }
}

/// The last generation handed out. Starts at zero, which no claim receives.
static GENERATIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Progress {
    /// A document for a run that has just been claimed and has not yet done
    /// anything.
    ///
    /// The ONE way to build a `Progress` that reads as in-flight, so a caller
    /// cannot claim the slot with a document that says nothing is happening.
    #[must_use]
    pub fn claimed() -> Self {
        Self {
            started: true,
            generation: GENERATIONS
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                .wrapping_add(1),
            ..Self::default()
        }
    }

    /// A claimed document stamped with when it was claimed and what its legs
    /// ask for (DB-B32): the constructor `POST /pull/run` uses.
    #[must_use]
    pub fn claimed_for(legs: &[Leg], now: std::time::SystemTime) -> Self {
        Self {
            started_at_ms: now
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|since| u64::try_from(since.as_millis()).ok()),
            asked: legs.iter().map(AskedLeg::of).collect(),
            ..Self::claimed()
        }
    }

    /// Whether a run is on this site right now.
    ///
    /// BOTH halves are load-bearing: never started, and started-then-finished,
    /// are both "not running", and only one of them is `finished.is_some()`.
    #[must_use]
    pub fn running(&self) -> bool {
        self.started && self.finished.is_none()
    }

    /// The status document, hand-written for the same reason every other JSON
    /// in this crate is: one dependency fewer, and the shape is pinned by the
    /// test that reads it rather than by a derive nobody looks at.
    #[must_use]
    pub fn json(&self) -> String {
        let mut out = String::with_capacity(256 + self.feeds.len() * 128);
        out.push_str("{\"running\":");
        out.push_str(if self.running() { "true" } else { "false" });
        out.push_str(",\"passes\":");
        out.push_str(&self.passes.to_string());
        out.push_str(",\"retries\":");
        out.push_str(&self.retries.to_string());
        out.push_str(",\"rowsAtStart\":");
        out.push_str(
            &self
                .rows_at_start
                .map_or_else(|| "null".to_owned(), |rows| rows.to_string()),
        );
        out.push_str(",\"rowsNow\":");
        out.push_str(
            &self
                .rows_now
                .map_or_else(|| "null".to_owned(), |rows| rows.to_string()),
        );
        out.push_str(",\"stopping\":");
        out.push_str(if self.stopping { "true" } else { "false" });
        out.push_str(",\"finished\":");
        match &self.finished {
            Some(word) => out.push_str(&quote_for_json(word)),
            None => out.push_str("null"),
        }
        // THE RUN'S OWN START AND ASK (DB-B32): a page that picks this run up
        // after a reload states these, never its own clock and selection.
        // THE CEILING AND THE TYPED STOP (DB-B26): "pass N of 400" and why the
        // run ended, as fields a page reads rather than a sentence it parses.
        out.push_str(",\"passCeiling\":");
        out.push_str(&MAX_PASSES.to_string());
        out.push_str(",\"stopReason\":");
        match self.stop {
            Some(stop) => out.push_str(&quote_for_json(stop.code())),
            None => out.push_str("null"),
        }
        out.push_str(",\"startedAtMs\":");
        out.push_str(
            &self
                .started_at_ms
                .map_or_else(|| "null".to_owned(), |ms| ms.to_string()),
        );
        out.push_str(",\"asked\":[");
        for (nth, leg) in self.asked.iter().enumerate() {
            if nth > 0 {
                out.push(',');
            }
            leg.json(&mut out);
        }
        out.push(']');
        out.push_str(",\"feeds\":[");
        for (nth, feed) in self.feeds.iter().enumerate() {
            if nth > 0 {
                out.push(',');
            }
            out.push_str("{\"vendor\":");
            out.push_str(&quote_for_json(&feed.vendor));
            out.push_str(",\"legs\":");
            out.push_str(&feed.legs.to_string());
            out.push_str(",\"legsDone\":");
            out.push_str(&feed.legs_done.to_string());
            out.push_str(",\"doing\":");
            out.push_str(&quote_for_json(&feed.doing));
            out.push_str(",\"retries\":");
            out.push_str(&feed.retries.to_string());
            out.push_str(",\"finished\":");
            out.push_str(if feed.finished { "true" } else { "false" });
            // ON THE WIRE, because the page has to be able to say WHY a feed
            // stopped while its siblings keep going. Without it a halted feed
            // and a finished one are the same two booleans, and the operator is
            // left to infer a credential death from a retry counter that has
            // stopped moving.
            out.push_str(",\"credentialDead\":");
            out.push_str(if feed.credential_dead {
                "true"
            } else {
                "false"
            });
            // ON THE WIRE FOR THE SAME REASON `credentialDead` IS: without it a
            // feed that deferred half its legs and a feed that had half as many
            // read identically on the page.
            out.push_str(",\"skipped\":");
            out.push_str(&feed.skipped.to_string());
            out.push_str(",\"standing\":");
            out.push_str(&feed.standing.to_string());
            out.push_str(",\"storeUnavailable\":");
            out.push_str(if feed.store_unavailable {
                "true"
            } else {
                "false"
            });
            out.push_str(",\"lastError\":");
            match &feed.last_error {
                Some(word) => out.push_str(&quote_for_json(word)),
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push_str("]}");
        out
    }
}

/// How many of the journal's newest records [`restart_scan`] reads:
/// **800**, four pages of [`crate::audit::MAX_PAGE_RECORDS`], about 205 KiB.
/// A bound on the read, not on the run: run records further back are read
/// through the journal's index, [`INTERRUPTED_RUNS`] of them (DB-B34).
pub const INTERRUPTED_SCAN: u64 = 4 * crate::audit::MAX_PAGE_RECORDS;

/// How many run records before the scan [`restart_scan`] reads through the
/// journal's index: **64**, one positional read of 256 bytes each. A run
/// opened further back than these and never closed is not reported, which
/// `docs/06-limits.md` names.
pub const INTERRUPTED_RUNS: usize = 64;

/// A pull run the journal opened and never closed (DB-B34).
///
/// A run's opening record is appended before its first instrument and its
/// closing record only when the run returns. A process that stopped in
/// between left the opening record and the failures behind it, and nothing
/// read them as a run cut short: after the restart `/pull/run.json` answered
/// exactly as a server that had never pulled.
///
/// **Only an opening record that was READ makes one of these.** Member
/// failures prove nothing about a run being open: every run of the build
/// before D-3341 wrote its failures BEHIND its closing record, and
/// `RunLedger::rest` still does when the opening record was refused, so a
/// journal that ends in 2.4 million failures can be a run that closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interrupted {
    /// The opening record's ordinal.
    pub ordinal: u64,
    /// When the run was opened.
    pub opened_at_secs: i64,
    /// The newest record of the run that was read: the last moment it is
    /// known to have been alive.
    pub last_at_secs: i64,
    /// What it was fetching from, as its opening record names it.
    pub source: String,
    /// The window it asked for, as days since the epoch.
    pub window: Option<(u32, u32)>,
    /// Member-failure records of the run among the records read: all of them
    /// when [`Self::failures_whole`], else those inside the scan.
    pub failures: u64,
    /// Whether every record after the opening record was read, so
    /// [`Self::failures`] is the run's whole count and not a floor.
    pub failures_whole: bool,
    /// The opening record's note ("opened for N instruments; ...").
    pub note: String,
}

impl Interrupted {
    /// The run in one sentence, for a page that shows nothing else.
    ///
    /// It does not say the SERVER stopped: a `cli` pull or a second server
    /// writes the same journal, and a run of theirs still in flight has no
    /// closing record either. What is known is said; which of the two it is,
    /// the age of the last record tells the reader.
    #[must_use]
    pub fn words(&self) -> String {
        let window = self.window.map_or_else(String::new, |(from, to)| {
            format!(" for {}..{}", day_text(from), day_text(to))
        });
        let failures = if self.failures_whole {
            format!("{} member failure(s)", self.failures)
        } else {
            format!(
                "at least {} member failure(s) (the newest {INTERRUPTED_SCAN} records were counted)",
                self.failures
            )
        };
        format!(
            "INTERRUPTED: a pull run{window} was opened at journal record {} ({} s since the \
             epoch) and has no closing record; the last record of it that was read was written \
             at {} s since the epoch, with {failures} on record after it. No run of this server \
             process is in flight, so the process that ran it stopped before closing it, unless \
             another process (a `cli` pull, a second server) is still running it. Bars it had \
             already written are in the store; press Pull with the same window to fetch what is \
             left.",
            self.ordinal, self.opened_at_secs, self.last_at_secs
        )
    }

    fn json(&self, out: &mut String) {
        out.push_str("{\"ordinal\":");
        out.push_str(&self.ordinal.to_string());
        out.push_str(",\"openedAtSecs\":");
        out.push_str(&self.opened_at_secs.to_string());
        out.push_str(",\"lastRecordAtSecs\":");
        out.push_str(&self.last_at_secs.to_string());
        out.push_str(",\"source\":");
        out.push_str(&quote_for_json(&self.source));
        out.push_str(",\"from\":");
        match self.window {
            Some((from, to)) => {
                out.push_str(&quote_for_json(&day_text(from)));
                out.push_str(",\"to\":");
                out.push_str(&quote_for_json(&day_text(to)));
            }
            None => out.push_str("null,\"to\":null"),
        }
        out.push_str(",\"failures\":");
        out.push_str(&self.failures.to_string());
        out.push_str(",\"failuresWhole\":");
        out.push_str(if self.failures_whole { "true" } else { "false" });
        out.push_str(",\"note\":");
        out.push_str(&quote_for_json(&self.note));
        out.push_str(",\"words\":");
        out.push_str(&quote_for_json(&self.words()));
        out.push('}');
    }
}

/// What the journal says, when a site loads, about the run before this
/// process (DB-B34): three answers, because "not interrupted" and "could not
/// tell" are different facts and the second was being served as a third,
/// false one ("INTERRUPTED ... The server stopped while it ran").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restart {
    /// Every run record read is closed, or none was read, AND every record
    /// read could be decoded. How far the scan reads is in [`Self::words`].
    Clean,
    /// An opening record was read and no closing record follows it.
    Interrupted(Interrupted),
    /// The newest run record could not be read, or a record that was read
    /// would not decode and may be a run's opening or closing record, so
    /// whether a run is open is NOT KNOWN; the reason, in its own words.
    NotKnown(String),
}

impl Restart {
    /// The run cut short, when one was found.
    #[must_use]
    pub const fn interrupted(&self) -> Option<&Interrupted> {
        match self {
            Self::Interrupted(run) => Some(run),
            Self::Clean | Self::NotKnown(_) => None,
        }
    }

    /// The state's word on `/pull/run.json`.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Interrupted(_) => "interrupted",
            Self::NotKnown(_) => "not_known",
        }
    }

    /// The state in one sentence.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::Clean => format!(
                "No pull run was left open as far as the scan looked: every run record it read \
                 is closed, or it read none, and every record it read could be decoded. It \
                 reads the journal's newest {INTERRUPTED_SCAN} records and, where the journal's \
                 index can be read, the {INTERRUPTED_RUNS} newest run records before them; a \
                 run left open further back than that is not reported."
            ),
            Self::Interrupted(run) => run.words(),
            Self::NotKnown(why) => format!(
                "NOT KNOWN whether a pull run was left open: {why}. Nothing is claimed either \
                 way; `/audit` lists the journal's runs."
            ),
        }
    }
}

/// A day since the epoch as `YYYY-MM-DD`, or the number when it is no date.
fn day_text(days: u32) -> String {
    pull::session::Day::from_days(days).map_or_else(|_| days.to_string(), |day| day.to_string())
}

/// The run the records `read` leave open, if any. `read` is journal records
/// oldest first, as `(ordinal, record)`: every run record from the oldest of
/// them on, and whichever member failures were read with them.
///
/// # The walk
///
/// One pass, oldest to newest.
///
/// * An opening record starts a run. A restated one ("continues the run
///   opened at record N", written when another writer appended in between)
///   rejoins the run it names; it carries the opening record's own source,
///   window and time, so it stands for an opening record that lies before
///   `read`. A restatement written AFTER its run closed (a failure whose first
///   append was refused, offered again behind the closing record) is not a
///   new run. A restatement of a run that IS open rejoins it whatever closed
///   before it: an earlier run's closing record of the same window and
///   source says nothing about this one.
/// * A closing run record ends the newest open run of its window and source,
///   else of its window: a run that reached no instrument is closed under its
///   target's name, not its endpoint's. A record of
///   [`crate::audit::Outcome::Refused`] closes nothing: it is a request
///   refused before anything ran, and no run opens with it, so another
///   request's refusal for the same window does not hide an open run.
/// * A member failure is counted to the run whose record it follows, and
///   NEVER opens one. Failures with no opening record before them are a
///   closed run's (the layout before D-3341, and `RunLedger::rest`'s when
///   the opening record was refused) or another run's; either way they are
///   no evidence that a run is open.
///
/// What is still open at the end was never closed.
///
/// # Cost
///
/// O(records read); the open list is bounded by the runs in flight at once,
/// which the feed seats bound.
#[must_use]
pub fn interrupted_in(read: &[(u64, crate::audit::Record)]) -> Option<Interrupted> {
    use crate::audit::{Kind, Outcome};
    let mut open: Vec<Interrupted> = Vec::new();
    // Opening ordinals of the runs a closing record ended, and the closing
    // records that ended no run that was read: a head restated after either
    // is not a new run.
    let mut closed: Vec<u64> = Vec::new();
    let mut closed_unseen: Vec<(Option<(u32, u32)>, String)> = Vec::new();
    let mut current: Option<u64> = None;
    for (ordinal, record) in read {
        let window = (record.from_days != 0 || record.to_days != 0)
            .then_some((record.from_days, record.to_days));
        match (record.kind, record.outcome) {
            (Kind::Run, Outcome::Opened) => {
                let restated = record
                    .note
                    .strip_prefix("continues the run opened at record ")
                    .and_then(|rest| rest.trim().parse::<u64>().ok());
                let first = restated.unwrap_or(*ordinal);
                if restated.is_some()
                    && !open.iter().any(|run| run.ordinal == first)
                    && (closed.contains(&first)
                        || closed_unseen
                            .iter()
                            .any(|(held, source)| *held == window && *source == record.source))
                {
                    current = None;
                    continue;
                }
                if !open.iter().any(|run| run.ordinal == first) {
                    open.push(Interrupted {
                        ordinal: first,
                        opened_at_secs: record.at_unix_secs,
                        last_at_secs: record.at_unix_secs,
                        source: record.source.clone(),
                        window,
                        failures: 0,
                        failures_whole: restated.is_none(),
                        note: if restated.is_some() {
                            String::new()
                        } else {
                            record.note.clone()
                        },
                    });
                }
                current = Some(first);
            }
            (Kind::Run, Outcome::Refused) => current = None,
            (Kind::Run, _) => {
                let ended = open
                    .iter()
                    .rposition(|run| run.window == window && run.source == record.source)
                    .or_else(|| open.iter().rposition(|run| run.window == window));
                match ended {
                    Some(at) => closed.push(open.remove(at).ordinal),
                    None => closed_unseen.push((window, record.source.clone())),
                }
                current = None;
            }
            (Kind::MemberFailure, _) => {
                if let Some(run) =
                    current.and_then(|first| open.iter_mut().find(|run| run.ordinal == first))
                {
                    run.failures = run.failures.saturating_add(1);
                    run.last_at_secs = run.last_at_secs.max(record.at_unix_secs);
                }
            }
        }
    }
    // The newest still-open run is the one that was cut short.
    open.pop()
}

/// What `journal` says about the run before this process (DB-B34). Asked
/// ONCE, when the site loads: nothing of this process is in flight then.
///
/// # What is read
///
/// 1. The journal's newest [`INTERRUPTED_SCAN`] records.
/// 2. When the journal is longer than that, the [`INTERRUPTED_RUNS`] newest
///    run records before them, each by its ordinal from the journal's own
///    index (`pull.journal.idx/runs`, one 32-byte entry per run record,
///    [`crate::audit::Journal::read_index`]). So a run whose opening record
///    lies two million failures back is still found with its true ordinal,
///    window and source, and a closing record that far back is still seen to
///    be one.
///
/// # An index that is absent or stale is built here, once
///
/// When the index is absent, of another file (a journal copied or restored
/// has a new inode) or short of the scan, the run records before the scan
/// cannot be named without walking the journal. The index is then built
/// where it is needed ([`crate::audit::Journal::build_index`]: one pass over
/// the journal under its append lock, the pass `api serve` starts in the
/// background anyway), so the answer is exact on the first start and not
/// only on the second. A lock another writer holds is waited for, briefly
/// ([`INDEX_WAITS`]).
///
/// # NOT KNOWN is an answer
///
/// If the index still cannot be read (the store will not take the write,
/// the lock stayed held) and the scan itself holds no run record that
/// settles it (an opening record, or a closing one), the answer is
/// [`Restart::NotKnown`] with both reasons. It was "INTERRUPTED ... The
/// server stopped while it ran", on a journal whose newest run record was a
/// closing one.
///
/// # A record that cannot be decoded is never dropped (repair 2)
///
/// Every record the scan is handed and cannot decode -- among the newest
/// [`INTERRUPTED_SCAN`], among the run records the index names, and among
/// the records the index's `faults` file names in the walk's span
/// ([`older_runs`]) -- is kept with its ordinal and the decoder's reason. If
/// one lies anywhere it could decide the answer, the answer is
/// [`Restart::NotKnown`] naming it: it is never "clean" (the damaged record
/// may be an opening record) and never "has no closing record" (it may be
/// that closing record). The one place it cannot decide is before the
/// opening record of the run the readable records leave open; that run is
/// still reported, and the damage is named on its log line.
///
/// # Cost
///
/// One `stat`, at most four page reads (800 records), one read of the
/// index's `runs` and `faults` files and at most 128 positional reads (64
/// run records, and the 64 newest filed faults among them): bounded whatever the
/// journal's length, except the `runs` file itself, which is 32 bytes per
/// run record ever written. When the index has to be built: one pass over
/// the whole journal, once per journal file, not per start.
#[must_use]
pub(crate) fn restart_scan(journal: &crate::audit::Journal) -> Restart {
    restart_scan_waiting(journal, INDEX_WAITS)
}

/// How many times [`restart_scan`] asks again for an index another writer
/// is building, [`INDEX_WAIT`] apart: **100**, ten seconds in all. The
/// journal's lock is held for one append, or for one index build (1.33 s
/// per GiB when it was measured, WP-33c), so this is a bound on a stuck
/// writer, not a wait anybody is expected to see.
const INDEX_WAITS: u32 = 100;

/// The pause between two of those.
const INDEX_WAIT: core::time::Duration = core::time::Duration::from_millis(100);

/// [`restart_scan`], waiting at most `waits` pauses for a held lock.
fn restart_scan_waiting(journal: &crate::audit::Journal, waits: u32) -> Restart {
    use crate::audit::{Kind, Log, Outcome};
    let log = journal.look();
    if let Log::Unreadable { reason } = &log {
        return not_known(format!("the pull journal cannot be read ({reason})"));
    }
    let records = log.records();
    if records == 0 {
        return Restart::Clean;
    }
    let mut tail: Vec<(u64, crate::audit::Record)> = Vec::new();
    // EVERY RECORD THE SCAN IS HANDED AND CANNOT DECODE IS KEPT, by ordinal
    // and reason. They were filtered out, and the scan then answered as if
    // it had read everything: "clean" of a journal whose opening record was
    // the damaged one, "has no closing record" of one whose closing record
    // was (DB-B34, repair 2).
    let mut damaged: Vec<Damaged> = Vec::new();
    let mut skip = 0_u64;
    while skip < INTERRUPTED_SCAN.min(records) {
        let page = match journal.page(records, skip, crate::audit::MAX_PAGE_RECORDS) {
            Ok(page) => page,
            Err(why) => return not_known(format!("the pull journal's newest records: {why}")),
        };
        if page.is_empty() {
            break;
        }
        skip = skip.saturating_add(page.len() as u64);
        for entry in page {
            match entry.decoded {
                Ok(record) => tail.push((entry.ordinal, record)),
                Err(fault) => damaged.push((entry.ordinal, fault.to_string())),
            }
        }
    }
    // `page` is newest first; the walk is oldest first.
    tail.reverse();
    let scan_first = records.saturating_sub(INTERRUPTED_SCAN);
    // THE RUN RECORDS BEFORE THE SCAN, through the index; `unread` is why
    // they could not be read, when they could not.
    let mut read: Vec<(u64, crate::audit::Record)> = Vec::new();
    let mut unread: Option<String> = None;
    if scan_first > 0 {
        match older_runs_built(journal, records, scan_first, waits) {
            Ok(older) => {
                read = older.read;
                damaged.extend(older.damaged);
            }
            Err(why) => unread = Some(why),
        }
    }
    read.extend(tail);
    damaged.sort_by_key(|(ordinal, _)| *ordinal);
    damaged.dedup_by_key(|(ordinal, _)| *ordinal);
    // A run record that settles the question was read: an opening record, or
    // a closing one. A refusal settles nothing.
    let settled = read
        .iter()
        .any(|(_, record)| record.kind == Kind::Run && record.outcome != Outcome::Refused);
    let found = interrupted_in(&read);
    // WHICH OF THE DAMAGED RECORDS PUT THE ANSWER IN DOUBT. A record that
    // would not decode may be a run's opening record or its closing record.
    // Before the opening record of the run the readable records leave open
    // it can be neither that run's close (a close follows its opening) nor a
    // newer open run (the newest is the one reported), so the answer stands
    // and the damage is named on its log line. Anywhere else -- after that
    // opening record, or anywhere at all when the readable records leave no
    // run open -- it may be the record that decides, and the answer is NOT
    // KNOWN.
    let opened = found.as_ref().map(|run| run.ordinal);
    let (before, doubt): (Vec<Damaged>, Vec<Damaged>) = damaged
        .into_iter()
        .partition(|(ordinal, _)| opened.is_some_and(|open| *ordinal <= open));
    if !doubt.is_empty() {
        return not_known(damaged_words(&doubt, found.as_ref()));
    }
    match found {
        Some(mut run) => {
            // The count is the run's whole count only when every record
            // after its opening record was read.
            run.failures_whole = run.failures_whole && run.ordinal >= scan_first;
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::error("pull.run", "a pull run has no closing record")
                    .with("ordinal", telemetry::Value::Uint(run.ordinal))
                    .with("failures", telemetry::Value::Uint(run.failures))
                    .with("source", telemetry::Value::Str(&run.source))
                    .with(
                        "undecodable_before",
                        telemetry::Value::Uint(before.len() as u64),
                    )
                    .with("undecodable", telemetry::Value::Str(&damaged_list(&before)))
                    .with("reason", telemetry::Value::Str(&run.words())),
            );
            Restart::Interrupted(run)
        }
        None => match unread {
            Some(why) if !settled => not_known(format!(
                "the journal's newest {INTERRUPTED_SCAN} of {records} records hold no run \
                 record, and the run records before them could not be read through its index \
                 ({why})"
            )),
            _ => Restart::Clean,
        },
    }
}

/// A record the restart scan was handed, or was told of by the journal's
/// index, and could not decode: its ordinal, and why in the decoder's words.
type Damaged = (u64, String);

/// How many damaged records a sentence names one by one: **3**. The rest
/// are counted and the newest of them named, so the sentence is bounded
/// whatever the damage.
const DAMAGED_NAMED: usize = 3;

/// `damaged` (ascending) as a list: the first [`DAMAGED_NAMED`] with their
/// reasons, then how many more and the newest of them. Empty for none.
fn damaged_list(damaged: &[Damaged]) -> String {
    let mut out = damaged
        .iter()
        .take(DAMAGED_NAMED)
        .map(|(ordinal, why)| format!("record {ordinal} ({why})"))
        .collect::<Vec<_>>()
        .join(", ");
    if let (Some(more), Some((newest, _))) = (
        damaged.len().checked_sub(DAMAGED_NAMED).filter(|n| *n > 0),
        damaged.last(),
    ) {
        use core::fmt::Write as _;
        let _infallible = write!(out, " and {more} more, the newest record {newest}");
    }
    out
}

/// Why the answer is NOT KNOWN when `doubt` would not decode: which records,
/// why each could decide, and what the records that DID decode say, so the
/// reader is told the half that is known and not handed it as the answer.
fn damaged_words(doubt: &[Damaged], found: Option<&Interrupted>) -> String {
    let readable = found.map_or_else(
        || "The records that could be decoded leave no run open".to_owned(),
        |run| {
            format!(
                "The records that could be decoded leave the run opened at record {} open, \
                 and a record after it that cannot be decoded may be its closing record",
                run.ordinal
            )
        },
    );
    format!(
        "{} of the journal records the scan read cannot be decoded -- {} -- and a record \
         that cannot be decoded may be a run's opening record or its closing record. {readable}",
        doubt.len(),
        damaged_list(doubt)
    )
}

/// [`Restart::NotKnown`] for `why`, logged where an operator reads it.
fn not_known(why: String) -> Restart {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.run",
            "whether a pull run was left open is not known",
        )
        .with("reason", telemetry::Value::Str(&why)),
    );
    Restart::NotKnown(why)
}

/// What [`older_runs`] read before the scan.
struct Older {
    /// The records that decoded, oldest first.
    read: Vec<(u64, crate::audit::Record)>,
    /// The records that did not, or that the index files as undecodable and
    /// were not read again.
    damaged: Vec<Damaged>,
}

/// [`older_runs`], building the journal's index first when it cannot be read
/// as it is. See [`restart_scan`].
///
/// # Errors
///
/// Why the index could not be read, and why it could not be built here:
/// the build failed, or another writer held the journal's lock for `waits`
/// pauses.
fn older_runs_built(
    journal: &crate::audit::Journal,
    records: u64,
    scan_first: u64,
    waits: u32,
) -> Result<Older, String> {
    let first = match older_runs(journal, records, scan_first) {
        Ok(read) => return Ok(read),
        Err(why) => why,
    };
    let mut waited = 0_u32;
    loop {
        match journal.build_index() {
            Ok(built) if !built.busy => break,
            // ANOTHER WRITER HOLDS THE LOCK: an append, or the index build
            // `api serve` started. Either leaves the index readable.
            Ok(_) if waited < waits => {
                waited = waited.saturating_add(1);
                std::thread::sleep(INDEX_WAIT);
                if let Ok(read) = older_runs(journal, records, scan_first) {
                    return Ok(read);
                }
            }
            Ok(_) => {
                return Err(format!(
                    "{first}; it was not built here because another writer held the journal's \
                     lock through {waited} wait(s) of {} ms",
                    INDEX_WAIT.as_millis()
                ));
            }
            Err(why) => return Err(format!("{first}; and it could not be built here: {why}")),
        }
    }
    older_runs(journal, records, scan_first)
        .map_err(|why| format!("{first}; it was rebuilt, and then: {why}"))
}

/// The [`INTERRUPTED_RUNS`] newest run records of `journal` below record
/// `scan_first`, oldest first, read through its index -- and every record in
/// their span that would not decode.
///
/// # A record that cannot be decoded is returned, never dropped
///
/// Two ways one is met, and both are reported in [`Older::damaged`]:
///
/// * the index's `runs` file names a record and the record does not decode
///   (it was damaged after it was indexed);
/// * the index's `faults` file names a record between the oldest run record
///   read and the scan (it was damaged BEFORE the index was built, so `runs`
///   does not name it and it may be a run record all the same). The newest
///   [`INTERRUPTED_RUNS`] of those are read again, since a later build may
///   decode what an earlier one could not; older ones are reported as the
///   index filed them.
///
/// The span begins at the oldest run record read, or at the journal's first
/// record when every run record below the scan was read: a fault further
/// back than the walk looks is outside it, as an open run there is.
///
/// # Errors
///
/// The index is absent, of another file, ahead of the journal, covers fewer
/// than `scan_first` records (run records between its cover and the scan
/// would be missed) or holds an entry that cannot be read (a run record may
/// be missing from it), or the journal cannot be read at a named record: by
/// name.
///
/// # Cost
///
/// One read of the index and at most `2 * INTERRUPTED_RUNS` positional
/// reads, whatever the journal's length.
fn older_runs(
    journal: &crate::audit::Journal,
    records: u64,
    scan_first: u64,
) -> Result<Older, String> {
    let index = journal.read_index(records)?;
    if index.covered < scan_first {
        return Err(format!(
            "the index covers the first {} records and the scan begins at record {scan_first}",
            index.covered
        ));
    }
    if index.malformed > 0 {
        return Err(format!(
            "{} of the index's entries cannot be read, so a run record may be missing from it",
            index.malformed
        ));
    }
    let below = index
        .runs
        .partition_point(|(ordinal, _)| *ordinal < scan_first);
    let named = index
        .runs
        .get(below.saturating_sub(INTERRUPTED_RUNS)..below)
        .unwrap_or_default();
    // WHERE THE WALK BEGINS: the oldest run record read, or the journal's
    // first record when no run record below the scan was left unread.
    let begins = if below > INTERRUPTED_RUNS {
        named.first().map_or(0, |(ordinal, _)| *ordinal)
    } else {
        0
    };
    let filed = index
        .fault_ordinals
        .get(
            index.fault_ordinals.partition_point(|at| *at < begins)
                ..index.fault_ordinals.partition_point(|at| *at < scan_first),
        )
        .unwrap_or_default();
    let (as_filed, again) = filed.split_at(filed.len().saturating_sub(INTERRUPTED_RUNS));
    let mut ordinals: Vec<u64> = named
        .iter()
        .map(|(ordinal, _)| *ordinal)
        .chain(again.iter().copied())
        .collect();
    ordinals.sort_unstable();
    ordinals.dedup();
    let mut out = Older {
        read: Vec::with_capacity(ordinals.len()),
        damaged: as_filed
            .iter()
            .map(|at| {
                (
                    *at,
                    "the journal's index files it as a record that would not decode".to_owned(),
                )
            })
            .collect(),
    };
    for (ordinal, decoded) in journal.records_at(&ordinals)? {
        match decoded {
            Ok(record) => out.read.push((ordinal, record)),
            Err(fault) => out.damaged.push((ordinal, fault.to_string())),
        }
    }
    Ok(out)
}

/// `/pull/run.json`: the run's document, the calendar of record's horizon
/// seen from `today` (days since the epoch; DB-B25, DB-B41) and what the
/// journal said about the run before this process when the site loaded
/// (DB-B34): `interrupted` is that run or `null`, and `restart` says which of
/// clean, interrupted and not known it is, so a `null` is never read as
/// "clean" when the scan could not tell.
///
/// The horizon is on this document because a pull is what the record's end
/// breaks: a page that polls the run can warn "the calendar is known until
/// <date>" before the first pull fails, with no second request.
#[must_use]
pub fn status_json(progress: &Progress, today: Option<i64>, restart: Option<&Restart>) -> String {
    let mut out = progress.json();
    let _brace = out.pop();
    out.push_str(",\"calendar\":");
    match today {
        Some(today) => {
            let horizon = pull::calendar::record_horizon(today);
            out.push_str("{\"recordTo\":");
            out.push_str(&quote_for_json(&day_text(
                u32::try_from(horizon.last_day).unwrap_or(0),
            )));
            out.push_str(",\"daysLeft\":");
            out.push_str(&horizon.days_left.to_string());
            out.push_str(",\"endsSoon\":");
            out.push_str(if horizon.ends_soon { "true" } else { "false" });
            out.push_str(",\"ended\":");
            out.push_str(if horizon.ended { "true" } else { "false" });
            out.push('}');
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"interrupted\":");
    match restart.and_then(Restart::interrupted) {
        Some(run) => run.json(&mut out),
        None => out.push_str("null"),
    }
    // THE THIRD STATE (DB-B34): `interrupted: null` alone could not tell "no
    // run was left open" from "could not tell".
    out.push_str(",\"restart\":");
    match restart {
        Some(state) => {
            out.push_str("{\"state\":");
            out.push_str(&quote_for_json(state.word()));
            out.push_str(",\"words\":");
            out.push_str(&quote_for_json(&state.words()));
            out.push('}');
        }
        None => out.push_str("null"),
    }
    out.push('}');
    out
}

/// A JSON string literal, escaping what RFC 8259 requires and nothing else.
///
/// Control characters below 0x20 are emitted as `\u00XX` rather than dropped:
/// a vendor message carrying one would otherwise produce a document no parser
/// accepts, and a status page that will not parse is indistinguishable from a
/// server that has died -- which is precisely the confusion this whole module
/// exists to remove.
#[must_use]
pub fn quote_for_json(raw: &str) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // `write!` rather than `push_str(&format!(..))`: one allocation
                // fewer, and clippy denies the latter. Writing into a `String`
                // cannot fail, so the result is deliberately discarded.
                let _cannot_fail = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Why a run was not started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The form carried no `leg` field at all.
    NothingAsked,
    /// A `leg` field did not carry five parts, or named a route this build
    /// does not serve.
    Malformed(String),
    /// A run is already in flight. Named rather than queued: two runs over one
    /// store would interleave two vendors' writes into one month file.
    AlreadyRunning,
    /// A leg's envelope names a vendor or rung its own payload does not ask
    /// for. The chain, the ladder and the failure gating all read the
    /// envelope while the route runs the payload, so the two must be one
    /// request (P1-02-01, D-1765).
    Disagrees {
        /// The leg, decoded once, as it arrived.
        leg: String,
        /// Which half disagreed, and with what.
        why: String,
    },
    /// The form carried more legs than one press may ([`MAX_RUN_LEGS`]).
    TooManyLegs(usize),
}

impl Refusal {
    /// What to tell the operator, in a sentence that says what to do.
    #[must_use]
    pub fn why(&self) -> String {
        match self {
            Self::NothingAsked => "The run carried no legs, so nothing was started. \
                 Tick at least one instrument, one segment and one timeframe."
                .to_owned(),
            Self::Malformed(part) => format!(
                "A leg could not be read and NOTHING was started -- a run that \
                 dropped the leg it could not parse would report success over a \
                 window it never asked for. The leg was: {part}"
            ),
            Self::AlreadyRunning => "A run is already in flight on this server. \
                 It is not queued behind this one: two runs over one store would \
                 interleave two vendors' writes into a single month file. Watch \
                 /pull/run.json, or stop it first."
                .to_owned(),
            Self::Disagrees { leg, why } => format!(
                "A leg's envelope disagrees with its own payload and NOTHING was \
                 started -- the run would be filed, ordered and gated under the \
                 envelope while the payload decides what is fetched. {why}. The \
                 leg was: {leg}"
            ),
            Self::TooManyLegs(count) => format!(
                "The run carried {count} legs and NOTHING was started. One press \
                 carries at most {MAX_RUN_LEGS}: one per feed, per rung, per \
                 route. Split the selection into two presses."
            ),
        }
    }
}

/// Reads the legs a run was asked for out of the form body.
///
/// # The wire shape, and why the body is encoded twice
///
/// One `leg` field per leg, each `route|vendor|dir|label|body`. The body is
/// percent-encoded a SECOND time by the page before the field itself is
/// encoded, so a form body containing `&` or `|` -- and every one of them does
/// -- cannot be mistaken for a separator. Decoding therefore runs twice on the
/// last part and once on the rest.
///
/// # Why a malformed leg refuses the whole run
///
/// Because the alternative is the failure class this repository has now fixed
/// four times: dropping the item you could not handle and reporting success for
/// the rest. A run that silently omitted one instrument would be reported to
/// the operator as a completed window.
///
/// # Errors
///
/// [`Refusal::NothingAsked`] for a body with no `leg`, and
/// [`Refusal::Malformed`] naming the first leg that could not be read,
/// whose route is not served, or whose vendor is no feed this build has.
///
/// # Cost
///
/// One pass over the body. O(n) in its length and nothing worse -- there is no
/// per-leg scan of the legs already read. Each leg's vendor is compared
/// with the fixed feed list, `pull::vendor::FEED_COUNT` names long.
pub fn legs_from(body: &str) -> Result<Vec<Leg>, Refusal> {
    let mut legs = Vec::new();
    for field in body.split('&') {
        let Some(("leg", raw)) = field.split_once('=') else {
            continue;
        };
        if legs.len() == MAX_RUN_LEGS {
            let count = body
                .split('&')
                .filter(|field| field.starts_with("leg="))
                .count();
            return Err(Refusal::TooManyLegs(count));
        }
        let decoded = percent_decode(raw);
        let mut parts = decoded.splitn(5, '|');
        let (Some(route), Some(vendor), Some(dir), Some(label), Some(payload)) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            return Err(Refusal::Malformed(decoded));
        };
        let Some(route) = Route::parse(route) else {
            return Err(Refusal::Malformed(decoded));
        };
        // A vendor this build does not have is a wiring fault, named like an
        // unknown route. It is also what keeps `by_feed`'s group count, and
        // so `conduct`'s chain count, bounded by the feeds rather than by the
        // distinct strings a form can carry. D-0906.
        if !pull::vendor::Feed::ALL
            .iter()
            .any(|feed| feed.wire() == vendor)
        {
            return Err(Refusal::Malformed(decoded));
        }
        // A LEG'S PAYLOAD IS A FORM TOO, and it reaches `pull_spot` and
        // `pull_fno` without passing the `one_value_per_form_field` middleware, so a repeated
        // single-value field inside it is refused here, before any leg runs
        // (h-api-2, D-1512), with the middleware's own bound on distinct keys
        // (P5-05, D-2659).
        let body = percent_decode(payload);
        match crate::server::form_key_verdict(&body) {
            Some(crate::server::FormKeys::Repeated(key)) => {
                return Err(Refusal::Malformed(format!(
                    "{decoded} (its form names {key:?} more than once)"
                )));
            }
            Some(crate::server::FormKeys::TooMany) => {
                return Err(Refusal::Malformed(format!(
                    "{decoded} (its form names more than {} distinct fields)",
                    crate::server::MAX_DISTINCT_FORM_KEYS
                )));
            }
            None => {}
        }
        if let Some(why) = envelope_disagreement(route, vendor, dir, &body) {
            return Err(Refusal::Disagrees { leg: decoded, why });
        }
        legs.push(Leg {
            route,
            vendor: vendor.to_owned(),
            dir: dir.to_owned(),
            label: label.to_owned(),
            body,
        });
    }
    if legs.is_empty() {
        return Err(Refusal::NothingAsked);
    }
    Ok(legs)
}

/// Where a leg's envelope and its payload name different requests, or `None`
/// when they are one.
///
/// The payload is read the way the route that runs it reads it: `vendor` through
/// [`crate::ingest::parse_feed`] (absent means Dhan), and for spot
/// `granularity` through [`crate::ingest::parse_granularity`] (absent means one
/// minute), for derivatives `series` through [`crate::ingest::Series`]. That is
/// the check `recovery::plan` already made on its own legs; `/pull/run`
/// trusted the label (P1-02-01, D-1765).
fn envelope_disagreement(route: Route, vendor: &str, dir: &str, body: &str) -> Option<String> {
    let asked = crate::server::param(body, "vendor");
    let feed = crate::ingest::parse_feed(&asked).map(pull::vendor::Feed::wire);
    if feed != Some(vendor) {
        return Some(format!(
            "the envelope names vendor {vendor:?} and the payload asks for {asked:?}"
        ));
    }
    let (field, payload_dir) = match route {
        Route::Spot => {
            let raw = crate::server::param(body, "granularity");
            let rung = crate::ingest::parse_granularity(&raw).map(pull::vendor::Granularity::dir);
            ("granularity", (raw, rung))
        }
        Route::Fno => {
            let raw = crate::server::param(body, "series");
            let series = crate::ingest::Series::from_slug(&raw).map(|series| match series {
                crate::ingest::Series::Futures => "futures",
                crate::ingest::Series::Options => "options",
            });
            ("series", (raw, series))
        }
    };
    let (raw, rung) = payload_dir;
    (rung != Some(dir))
        .then(|| format!("the envelope names rung {dir:?} and the payload's {field} is {raw:?}"))
}

/// Groups legs by vendor, keeping the order the feeds were ticked, and sorts
/// each group onto the ladder.
///
/// # Cost
///
/// The group lookup scans the vendors found so far, which is bounded by the
/// number of feeds this build has (`pull::vendor::FEED_COUNT`) rather than by
/// the number of legs, for legs [`legs_from`] admitted: it refuses a vendor
/// that is no feed (D-0906). Legs built by hand are not checked here. The
/// sort is per group and `sort_by_key` is stable, so two legs on one rung
/// keep the order the operator ticked them in.
#[must_use]
pub fn by_feed(legs: Vec<Leg>) -> Vec<(String, Vec<Leg>)> {
    let mut groups: Vec<(String, Vec<Leg>)> = Vec::new();
    for leg in legs {
        match groups.iter_mut().find(|(vendor, _)| *vendor == leg.vendor) {
            Some((_, held)) => held.push(leg),
            None => groups.push((leg.vendor.clone(), vec![leg])),
        }
    }
    for (_, held) in &mut groups {
        held.sort_by_key(|leg| ladder_rank(&leg.dir));
    }
    groups
}

/// Total rows the store holds right now, across every vendor's census.
///
/// **Fresh, never the startup snapshot.** `census_now` re-reads the manifests,
/// which is the only reading of progress that cannot be fooled by a run
/// reporting bars it did not commit. That distinction is the whole basis of the
/// pass loop: a pass is idle when the STORE did not grow, not when a receipt
/// says so.
///
/// # Cost
///
/// NOT O(1) on a miss. On a cache hit, five `stat`s (six when no manifest
/// answered) and no manifest read. On the first call after any vendor's
/// manifest moved, a whole `census::read_all` of all five vendors and a sort
/// of every held entry: O(manifest bytes + E log E), growing with the store.
/// During a pull every committed leg moves a manifest, so the conductor's
/// calls (start, before and after each pass, every [`ROWS_TICK`], end) and
/// recovery's call after each attempt miss. Outside the per-leg path; the
/// census a miss builds is shared with every other `census_now` caller.
/// `docs/06-limits.md` "Pull-run and recovery row counts (D-1382)" says why a
/// header-only read is not used. UNVERIFIED: the bound is read from the
/// code and has not been measured.
pub(crate) fn rows_now(site: &Site) -> Option<u64> {
    let (censuses, _) = crate::server::census_now(site);
    rows_total(
        censuses
            .iter()
            .filter_map(census::VendorCensus::counters)
            .map(|(_months, rows, _entries)| rows),
    )
}

/// The rows the store's census holds for `own`, the store vendors a run's
/// own feeds write to — the run's growth test and its "bars added" figure.
///
/// # Why not [`rows_now`] (DB-B33)
///
/// That sums every vendor. A GDFL import writing beside a Zerodha pull moved
/// the total on every pass, so the run read another writer's rows as its own
/// progress: it never stopped idle, skipped [`RETRY_WAIT`] on growth that was
/// not its own, and reported the other feed's bars as "added" by the pull.
/// A run's growth is measured on the vendors it writes to and no other.
///
/// # Cost
///
/// As [`rows_now`]: the shared census read, then one comparison per vendor
/// against a list bounded by the feed count.
pub(crate) fn rows_of(site: &Site, own: &[brutex_core::vendor::Vendor]) -> Option<u64> {
    let (censuses, _) = crate::server::census_now(site);
    rows_total(
        censuses
            .iter()
            .filter(|census| own.contains(&census.vendor))
            .filter_map(census::VendorCensus::counters)
            .map(|(_months, rows, _entries)| rows),
    )
}

/// The store vendors the feeds of `groups` write to, each once. A feed with
/// no store vendor of its own contributes none, so its run proves no growth
/// rather than borrowing another vendor's.
fn own_vendors(groups: &[(String, Vec<Leg>)]) -> Vec<brutex_core::vendor::Vendor> {
    let mut own = Vec::with_capacity(groups.len());
    for (wire, _) in groups {
        let vendor = pull::vendor::Feed::ALL
            .iter()
            .find(|feed| feed.wire() == wire)
            .and_then(|feed| feed.store_vendor());
        if let Some(vendor) = vendor
            && !own.contains(&vendor)
        {
            own.push(vendor);
        }
    }
    own
}

/// The vendors' row counts added, or `None` when the total does not fit.
///
/// A plain `.sum()` here panicked on overflow, and the release profile aborts
/// on a panic: two CRC-valid manifests each claiming about `u64::MAX` rows
/// (`Entry::check` refuses only zero) killed the server on the first pull
/// POST and on every restart after it (CE-74, D-1776). The page shows the
/// count as unknown instead, and the pass loop treats an unknown count as no
/// proven growth.
fn rows_total(mut rows: impl Iterator<Item = u64>) -> Option<u64> {
    rows.try_fold(0_u64, u64::checked_add)
}

/// Edits the live progress, if run `run` still owns the slot.
///
/// A closure rather than a returned guard, so the lock cannot be held across an
/// `.await` by accident — which is the one way a `std::sync::Mutex` here could
/// stall the whole fan-out.
///
/// The edit lands only on the document whose [`Progress::generation`] is
/// `run`. Once the slot holds a later claim, a late write from this run (its
/// `Finisher`, a ticker poll that was mid-flight at `abort`, a detached chain)
/// is dropped rather than written into someone else's run. D-2760.
fn with_progress<F: FnOnce(&mut Progress)>(site: &Site, run: u64, edit: F) {
    let mut held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(progress) = held.as_mut()
        && progress.generation == run
    {
        edit(progress);
    }
}

/// Which feeds have halted on a dead credential, by position.
///
/// # Why a snapshot and not a probe per feed
///
/// The spawn loop needs the answer for every feed and takes the lock once to get
/// it, rather than once per feed inside the loop. Bounded by the feed count —
/// five today, capped at eight — so it is one small `Vec` per pass and no
/// allocation that grows with legs, months or passes.
///
/// Read BEFORE the pass clears `finished`, because a halted feed must keep its
/// `finished` flag: the page otherwise draws it as pending forever while nothing
/// is ever spawned for it.
fn halted_feeds(site: &Site, run: u64) -> Vec<bool> {
    let held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.as_ref()
        .filter(|progress| progress.generation == run)
        .map_or_else(Vec::new, |progress| {
            progress
                .feeds
                .iter()
                .map(|feed| feed.credential_dead)
                .collect()
        })
}

/// Whether the operator has asked this run to stop.
///
/// Checked between legs rather than inside one: a leg that has already asked
/// the vendor for bars must be allowed to write them, or a stop would throw
/// away answers that were already paid for.
///
/// A run whose slot now holds another claim (or nothing) is stopped too: it
/// no longer has a document to report into, and pulling on would put a second
/// run's writes on one store behind the first one's back. D-2760.
fn stopping(site: &Site, run: u64) -> bool {
    let held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.as_ref()
        .is_none_or(|progress| progress.generation != run || progress.stopping)
}

/// Marks the run finished HOWEVER the task ends.
///
/// # The deadlock this exists to stop
///
/// `finished: None` is the only reading of "a run is in flight", and
/// `/pull/run` refuses to start a second one while it holds. A task that
/// panicked, or that was cancelled at a shutdown, would leave that `None` in
/// place forever and every later press would be refused with `AlreadyRunning`
/// against a run that no longer exists. `Drop` runs when the task is cancelled
/// or dropped at shutdown, in every build, so the slot is released on those
/// exits as well as the happy one. A panic is covered only where it unwinds:
/// `dev` and `test`. `release` sets `panic = "abort"` (root `Cargo.toml`), so
/// there a panic ends the process and the slot dies with it (poison-1, D-1771).
///
/// It writes only when nothing else has: a run that finished normally has
/// already put its own summary there, and this must not paint over it. And it
/// writes only into ITS run's document: the slot is free from the moment the
/// summary lands, and a press admitted before this drop has a `finished: None`
/// of its own that this must not read as ours. D-2760.
struct Finisher {
    /// Whose slot to release.
    site: Loaded,
    /// Which claim of that slot is ours.
    run: u64,
}

impl Drop for Finisher {
    fn drop(&mut self) {
        with_progress(&self.site, self.run, |progress| {
            if progress.finished.is_none() {
                progress.finished = Some(
                    "The run ended without recording a summary, which means the task \
                     carrying it stopped abnormally — a panic, or the server shutting \
                     down under it. Nothing already written to the store is affected; \
                     press Pull again to start a new run. Leg checkpoints exist \
                     only in memory and are not restored after a restart."
                        .to_owned(),
                );
            }
        });
    }
}

/// The result of a feed's last pass. Kept inside the conductor so a terminal
/// refusal cannot be mistaken for a clean pass when that feed is skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PassOutcome {
    Clean,
    Retry,
    Halted,
}

/// A clean receipt checkpoints only this leg, only for the current retry
/// cycle. Shared with the conductor so a later request panicking cannot lose
/// earlier receipts along with the task's return value. Relaxed atomics carry
/// no other data; the conductor joins every chain before the next pass.
///
/// THREE STATES, NOT TWO (DB-B26). [`OWED`] and [`CLEAN`] are the receipt
/// checkpoint above. [`SETTLED`] is a leg that finished with standing
/// failures: it is never reopened in this run, on a retry cycle or a full
/// pass, because asking again returns the same answer.
type Checkpoints = std::sync::Arc<[std::sync::atomic::AtomicU8]>;

/// A leg not yet answered clean in this cycle.
const OWED: u8 = 0;
/// A leg whose receipt read clean: reopened on the next full pass.
const CLEAN: u8 = 1;
/// A leg that finished with standing failures: never reopened in this run.
const SETTLED: u8 = 2;

fn checkpoints_for(groups: &[(String, Vec<Leg>)]) -> Vec<Checkpoints> {
    groups
        .iter()
        .map(|(_, legs)| {
            legs.iter()
                .map(|_| std::sync::atomic::AtomicU8::new(OWED))
                .collect()
        })
        .collect()
}

/// The receipt and HTTP status jointly decide whether another attempt helps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegOutcome {
    Stored,
    Empty,
    Retry,
    /// The receipt classes every failure as standing ([`RetryClass`]).
    Standing,
    Permanent,
    Credential,
    /// The receipt says the store will not take a write (DB-B35).
    Store,
}

/// These are statuses of our internal handlers, not raw vendor responses.
/// 404 is an unavailable route/resource and 422 the mapping/session preflight
/// refusal. A 400 is terminal only with an explicit REFUSED verdict: the server
/// also uses 400 for pre-wire failures, including temporary credential reads.
/// 409 stays retryable for a busy feed seat or an unfinished ladder prerequisite.
fn leg_outcome(status: axum::http::StatusCode, html: &str) -> LegOutcome {
    if matches!(status.as_u16(), 401 | 403)
        || (!status.is_success() && crate::autopilot::credential_fault_in_page(html))
    {
        return LegOutcome::Credential;
    }
    if matches!(status.as_u16(), 404 | 422) {
        return LegOutcome::Permanent;
    }
    // Read the first verdict badge, not any green badge elsewhere in a page.
    let badge = html
        .split_once("<span class=\"badge ")
        .and_then(|(_, rest)| rest.split_once("</span>"))
        .map(|(verdict, _)| verdict);
    if status == axum::http::StatusCode::BAD_REQUEST && badge == Some("bad\">REFUSED") {
        return LegOutcome::Permanent;
    }
    if status.is_success() {
        match badge {
            Some("good\">STORED") => return LegOutcome::Stored,
            Some("bad\">EMPTY") => return LegOutcome::Empty,
            _ => {}
        }
    }
    // Some handlers return HTTP 200 with a failed receipt. Its explicit
    // credential verdict must still halt the feed.
    if crate::autopilot::credential_fault_in_page(html) {
        return LegOutcome::Credential;
    }
    // THE RECEIPT'S OWN CLASS, WHEN IT STATES ONE (DB-B26, DB-B35). Every
    // receipt that was not clean used to be Retry, so a leg whose instruments
    // all answered and whose only failures were gaps in what the vendor holds
    // was pulled again whole, up to the pass ceiling. The server writes the
    // class where each fault is typed; a receipt that states none is Retry,
    // as before.
    match RetryClass::of_receipt(html) {
        Some(RetryClass::Store) => LegOutcome::Store,
        Some(RetryClass::Standing) => LegOutcome::Standing,
        Some(RetryClass::Transient) | None => LegOutcome::Retry,
    }
}

/// The production request boundary. Tests substitute receipts here while
/// exercising the same pass loop and feed state transitions, without sockets.
async fn request_leg(site: Loaded, leg: Leg) -> (axum::http::StatusCode, String) {
    match leg.route {
        Route::Spot => {
            let (status, _receipt, body) =
                crate::server::pull_spot(axum::extract::State(site), leg.body).await;
            (status, body.0)
        }
        Route::Fno => {
            let (status, body) =
                crate::server::pull_fno(axum::extract::State(site), leg.body).await;
            (status, body.0)
        }
    }
}

/// Record a failure without copying an HTML document into the wire status.
/// A terminal cause replaces an earlier transient error so the action needed
/// to resume is visible. Otherwise retain the first failure for diagnosis.
fn note_leg_failure(
    site: &Site,
    run: u64,
    nth: usize,
    leg: &Leg,
    (status, html): (axum::http::StatusCode, &str),
    outcome: LegOutcome,
) {
    // THE RECEIPT'S OWN REFUSAL, WHEN IT STATES ONE (DB-B37). A seat-busy 409
    // returns before any journal write, so "the reason is in the audit
    // journal" sent the operator to a record that was never written. The
    // receipt's `Refused because` line is the reason, and it is carried.
    //
    // WHETHER A RECORD EXISTS IS THE RECEIPT'S OWN `Recorded` LINE, never an
    // inference from the refusal: two spot receipts carry `Refused because`
    // AND a journal record (`server::recorded_fact` appends it: a blocked run,
    // and a mapping that reached nothing), and for those this said "no journal
    // record was written" about a record that exists. Only a receipt with no
    // `Recorded` fact (the seat 409, the unknown-feed 400: both return before
    // any append) is a door refusal.
    if let Some(said) = receipt_refusal(html) {
        let journal = match receipt_fact(html, "Recorded") {
            Some(recorded) => format!("the receipt's Recorded line: {recorded}"),
            None => "no journal record was written for a request refused at the door".to_owned(),
        };
        let kind = match outcome {
            LegOutcome::Retry => "this leg remains owed and is eligible for another pass",
            LegOutcome::Standing => {
                "no later pass of this run can change that, so this leg is not asked again"
            }
            _ => "this feed is halted for the rest of this run",
        };
        with_progress(site, run, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                if feed.last_error.is_none()
                    || matches!(
                        outcome,
                        LegOutcome::Credential | LegOutcome::Permanent | LegOutcome::Store
                    )
                {
                    feed.last_error = Some(format!(
                        "{} answered HTTP {}: {said} ({kind}; {journal})",
                        leg.label,
                        status.as_u16()
                    ));
                }
                feed.credential_dead |= outcome == LegOutcome::Credential;
                feed.store_unavailable |= outcome == LegOutcome::Store;
            }
        });
        return;
    }
    let reason = match outcome {
        LegOutcome::Credential => {
            "CREDENTIAL or authorization failure. This feed is halted \
            for the rest of this run. Refresh or correct access where it is managed, then \
            start a new pull; no token is minted here."
        }
        LegOutcome::Permanent => {
            "PERMANENT REFUSAL for this request. This feed is halted \
            for the rest of this run. Correct the request or preflight evidence before \
            starting a new pull. The refusal details are in the audit journal."
        }
        LegOutcome::Store => {
            "STORE UNAVAILABLE: the store would not take a write, so the leg stopped \
            asking the vendor. This feed is halted for the rest of this run. Check that \
            the store's drive is mounted and has room, then start a new pull; what was \
            asked and not written is asked again then."
        }
        LegOutcome::Standing => {
            "FINISHED WITH STANDING FAILURES: every failure left is one another attempt \
            in this run would repeat (what the vendor holds for an instrument that \
            answered, an instrument the pull order withheld, or the end of the calendar \
            of record). So this leg is not asked again in this run. Each failure is named \
            in the audit journal."
        }
        _ => {
            "its receipt did not read clean. The reason is in the audit journal; \
            this leg remains owed and is eligible for another pass."
        }
    };
    with_progress(site, run, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
            if feed.last_error.is_none()
                || matches!(
                    outcome,
                    LegOutcome::Credential | LegOutcome::Permanent | LegOutcome::Store
                )
            {
                feed.last_error = Some(format!(
                    "{} answered HTTP {}: {reason}",
                    leg.label,
                    status.as_u16()
                ));
            }
            feed.credential_dead |= outcome == LegOutcome::Credential;
            feed.store_unavailable |= outcome == LegOutcome::Store;
        }
    });
}

/// The `Refused because` line of a receipt page, unescaped, or `None` when
/// the receipt states none: the reason a request refused at the door (a held
/// seat, an unreadable feed) gives, which no journal record repeats. DB-B37.
fn receipt_refusal(html: &str) -> Option<String> {
    receipt_fact(html, "Refused because")
}

/// One fact of a receipt page by its label, unescaped, or `None` when the
/// receipt carries no such row (or an empty one). The first row wins, as the
/// first verdict badge does in [`leg_outcome`].
fn receipt_fact(html: &str, label: &str) -> Option<String> {
    let (_, rest) = html.split_once(&format!("<th>{label}</th><td>"))?;
    let (cell, _) = rest.split_once("</td>")?;
    let text = cell
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// Run one feed's legs sequentially. Only a response that actually returned
/// advances `legs_done`; dependencies and early breaks remain unattempted.
async fn run_chain<F, Fut>(
    site: Loaded,
    run: u64,
    nth: usize,
    legs: Vec<Leg>,
    checkpoints: Checkpoints,
    attempted: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    request: F,
) -> PassOutcome
where
    F: Fn(Loaded, Leg) -> Fut,
    Fut: core::future::Future<Output = (axum::http::StatusCode, String)>,
{
    let mut result = PassOutcome::Clean;
    let mut failed_spot_rank = None;
    for (leg, clean) in legs.iter().zip(checkpoints.iter()) {
        if stopping(&site, run) {
            break;
        }
        if clean.load(std::sync::atomic::Ordering::Relaxed) != OWED {
            continue;
        }
        // A failed daily leg holds back minutes/seconds, and any failed spot
        // rung holds back FNO. Other legs on the same rung may still progress.
        // Checkpointed prerequisites remain clean during failure retries.
        if failed_spot_rank
            .is_some_and(|rank| leg.route == Route::Fno || ladder_rank(&leg.dir) > rank)
        {
            continue;
        }
        with_progress(&site, run, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.doing.clone_from(&leg.label);
            }
        });
        attempted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (status, html) = request(Loaded::clone(&site), leg.clone()).await;
        with_progress(&site, run, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.legs_done = feed.legs_done.saturating_add(1);
            }
        });
        let outcome = leg_outcome(status, &html);
        if matches!(outcome, LegOutcome::Stored | LegOutcome::Empty) {
            clean.store(CLEAN, std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        note_leg_failure(&site, run, nth, leg, (status, &html), outcome);
        if outcome == LegOutcome::Standing {
            // SETTLED FOR THE RUN, AND THE LEGS AFTER IT STILL RUN (DB-B26,
            // DB-B27). The rung below is judged per instrument by the pull
            // order's own gate, so a daily leg with gaps no longer holds
            // back every minute; and nothing is retried, because every
            // instrument that could answer has.
            clean.store(SETTLED, std::sync::atomic::Ordering::Relaxed);
            with_progress(&site, run, |progress| {
                if let Some(feed) = progress.feeds.get_mut(nth) {
                    feed.standing = feed.standing.saturating_add(1);
                }
            });
            continue;
        }
        if leg.route == Route::Spot {
            failed_spot_rank = Some(ladder_rank(&leg.dir));
        }
        if matches!(
            outcome,
            LegOutcome::Credential | LegOutcome::Permanent | LegOutcome::Store
        ) {
            result = PassOutcome::Halted;
            break;
        }
        result = PassOutcome::Retry;
    }
    with_progress(&site, run, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
            feed.skipped = u32::try_from(legs.len())
                .unwrap_or(u32::MAX)
                .saturating_sub(feed.legs_done);
            feed.doing.clear();
            feed.finished = true;
        }
    });
    result
}

/// Run one pass, preserving terminal outcomes across passes. Retry counters
/// advance only when a failed feed actually attempts another request, including a
/// pass after a failure that also committed bars.
///
/// Reopen clean legs only after ALL retryable feeds recover (halted feeds stay
/// halted). A healthy sibling must not be repulled merely because another feed
/// failed. Repeated clean passes remain intentional: STORED/EMPTY receipts do
/// not attest complete coverage, and the existing incremental/idle policy asks
/// again until three passes add no rows. The tradeoff is that a previously
/// clean leg is not refreshed while a sibling is still retrying, even if its
/// data becomes available meanwhile. Nothing here provides restart resume.
/// These receipt checkpoints do not repair or certify source gaps. In
/// particular, HTTP 200 PARTIAL remains Retry: the entire unchanged failed leg
/// is eligible on every pass, even without growth, up to the run's pass ceiling.
/// There is no chunk checkpoint or new classification of its untyped prose.
async fn run_pass<F, Fut>(
    site: &Loaded,
    run: u64,
    groups: &[(String, Vec<Leg>)],
    outcomes: &mut [PassOutcome],
    checkpoints: &[Checkpoints],
    request: &F,
) where
    F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
    Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
{
    if stopping(site, run) {
        return;
    }
    let halted = halted_feeds(site, run);
    let retry_cycle = outcomes.contains(&PassOutcome::Retry);
    let mut flying = Vec::with_capacity(groups.len());
    for (nth, (((_vendor, group), prior), clean)) in groups
        .iter()
        .zip(outcomes.iter_mut())
        .zip(checkpoints)
        .enumerate()
    {
        if halted.get(nth).copied().unwrap_or(false) {
            *prior = PassOutcome::Halted;
        }
        if *prior == PassOutcome::Halted {
            continue;
        }
        if !retry_cycle {
            for leg in clean.iter() {
                // Only a clean receipt is reopened; a settled leg stays so.
                let _was = leg.compare_exchange(
                    CLEAN,
                    OWED,
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        }
        let again = *prior == PassOutcome::Retry;
        with_progress(site, run, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.finished = false;
                feed.legs_done = 0;
                feed.skipped = 0;
            }
        });
        let attempted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        flying.push((
            nth,
            again,
            std::sync::Arc::clone(&attempted),
            tokio::spawn(run_chain(
                Loaded::clone(site),
                run,
                nth,
                group.clone(),
                Checkpoints::clone(clean),
                attempted,
                request.clone(),
            )),
        ));
    }
    let mut retrying = false;
    for (nth, again, attempted, chain) in flying {
        let outcome = match chain.await {
            Ok(outcome) => outcome,
            Err(dead) => {
                note_dead_chain(
                    site,
                    run,
                    nth,
                    attempted.load(std::sync::atomic::Ordering::Relaxed),
                    &dead,
                );
                PassOutcome::Retry
            }
        };
        if again && attempted.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            retrying = true;
            with_progress(site, run, |progress| {
                if let Some(feed) = progress.feeds.get_mut(nth) {
                    feed.retries = feed.retries.saturating_add(1);
                }
            });
        }
        if let Some(held) = outcomes.get_mut(nth) {
            *held = outcome;
        }
    }
    if retrying {
        with_progress(site, run, |progress| {
            progress.retries = progress.retries.saturating_add(1);
        });
    }
}

/// Server-owned passes outlive the browser, but are not persisted across a
/// server restart. Feeds run in parallel; each feed's legs run sequentially.
/// Fixed refusals halt that feed. Transient failures retry up to [`MAX_PASSES`].
/// Idle termination is an observation about store growth, never full coverage.
///
/// `run` is the [`Progress::generation`] the caller installed when it claimed
/// the slot; every edit this run makes is addressed to it. D-2760.
pub async fn conduct(site: Loaded, run: u64, legs: Vec<Leg>) {
    conduct_with(site, run, legs, request_leg).await;
}

/// The request seam permits deterministic coordinator tests without vendors,
/// credentials, alternate production behavior, or a second pass loop.
async fn conduct_with<F, Fut>(site: Loaded, run: u64, legs: Vec<Leg>, request: F)
where
    F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
    Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
{
    let _finisher = Finisher {
        site: Loaded::clone(&site),
        run,
    };
    let groups = by_feed(legs);
    // THE RUN'S OWN VENDORS, AND NO OTHER WRITER'S ROWS (DB-B33).
    let own = own_vendors(&groups);
    let started_rows = rows_of(&site, &own);
    with_progress(&site, run, |progress| {
        progress.rows_at_start = started_rows;
        progress.rows_now = started_rows;
        progress.feeds = groups
            .iter()
            .map(|(vendor, held)| FeedReport {
                vendor: vendor.clone(),
                legs: u32::try_from(held.len()).unwrap_or(u32::MAX),
                ..FeedReport::default()
            })
            .collect();
    });

    let ticker = {
        let site = Loaded::clone(&site);
        let own = own.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(ROWS_TICK).await;
                let seen = rows_of(&site, &own);
                with_progress(&site, run, |progress| progress.rows_now = seen);
            }
        })
    };
    let mut outcomes = vec![PassOutcome::Clean; groups.len()];
    let checkpoints = checkpoints_for(&groups);
    let mut passes = 0_u32;
    let mut clean_empty = 0_u32;
    while passes < MAX_PASSES {
        if stopping(&site, run) {
            break;
        }
        let repairing = outcomes.contains(&PassOutcome::Retry);
        let before = rows_of(&site, &own);
        run_pass(&site, run, &groups, &mut outcomes, &checkpoints, &request).await;
        passes = passes.saturating_add(1);
        let after = rows_of(&site, &own);
        with_progress(&site, run, |progress| {
            progress.passes = passes;
            progress.rows_now = after;
        });
        if stopping(&site, run)
            || outcomes
                .iter()
                .all(|outcome| *outcome == PassOutcome::Halted)
        {
            break;
        }
        // Growth is proven only by two known counts; an unknown one (CE-74)
        // proves nothing, so the pass is judged as if nothing landed.
        if matches!((before, after), (Some(before), Some(after)) if after > before) {
            clean_empty = 0;
            continue;
        }
        if outcomes.contains(&PassOutcome::Retry) {
            clean_empty = 0;
            // No delay for a pass that can never be scheduled again.
            if passes < MAX_PASSES {
                tokio::time::sleep(RETRY_WAIT).await;
            }
            continue;
        }
        if repairing {
            // A repair pass reused earlier receipts, so it is not a fresh
            // observation of every eligible leg. Require full clean passes
            // for idle termination, including after a failure that added rows.
            clean_empty = 0;
            continue;
        }
        clean_empty = clean_empty.saturating_add(1);
        if clean_empty >= CLEAN_EMPTY_PASSES {
            break;
        }
    }
    // `abort` cannot stop a poll already inside the synchronous `rows_now`;
    // that poll would finish and write after the summary below. Awaiting the
    // handle returns only once the task has actually stopped. D-2760.
    ticker.abort();
    let _cancelled = ticker.await;
    let current_rows = rows_of(&site, &own);
    with_progress(&site, run, |progress| {
        progress.rows_now = current_rows;
        progress.stop = Some(stop_of(
            progress,
            &outcomes,
            clean_empty >= CLEAN_EMPTY_PASSES,
        ));
        progress.finished = Some(run_summary(
            progress,
            &outcomes,
            current_rows
                .zip(started_rows)
                .map(|(now, start)| now.saturating_sub(start)),
        ));
    });
}

/// Why the run ended, typed. The first that holds, in this order: the
/// operator's stop, a store that would not take a write, a halted feed, the
/// pass ceiling, legs that finished with standing failures, and otherwise the
/// idle stop. DB-B26, DB-B35.
fn stop_of(progress: &Progress, outcomes: &[PassOutcome], idle: bool) -> Stop {
    if progress.stopping {
        Stop::Operator
    } else if progress.feeds.iter().any(|feed| feed.store_unavailable) {
        Stop::StoreUnavailable
    } else if outcomes.contains(&PassOutcome::Halted)
        || progress.feeds.iter().any(|feed| feed.credential_dead)
    {
        Stop::Halted
    } else if progress.passes >= MAX_PASSES && !idle {
        // A run that went idle ON its last allowed pass stopped because it
        // was idle; the ceiling is the stop only of a run still owing.
        Stop::Ceiling
    } else if progress.feeds.iter().any(|feed| feed.standing > 0) {
        Stop::Standing
    } else {
        Stop::Idle
    }
}

/// The sentence a summary ends with when legs finished with standing
/// failures, or nothing. It names each such feed and its first reason, so the
/// stop is never read as a clean window (DB-B26).
fn standing_note(progress: &Progress) -> String {
    let named: Vec<String> = progress
        .feeds
        .iter()
        .filter(|feed| feed.standing > 0)
        .map(|feed| {
            format!(
                "{} ({} leg(s)): {}",
                feed.vendor,
                feed.standing,
                feed.last_error.as_deref().unwrap_or("standing failures")
            )
        })
        .collect();
    if named.is_empty() {
        String::new()
    } else {
        format!(
            " FINISHED WITH STANDING FAILURES, which were not retried because asking again \
             returns the same answer: {}",
            named.join("; ")
        )
    }
}

/// Terminal feeds remain part of the final verdict even while other feeds
/// reach an idle stop. Neither a skipped feed nor a stopped task is success.
fn run_summary(progress: &Progress, outcomes: &[PassOutcome], landed: Option<u64>) -> String {
    let landed_said = landed_words(landed);
    let halted: Vec<String> = progress
        .feeds
        .iter()
        .zip(outcomes)
        .filter(|(feed, outcome)| feed.credential_dead || **outcome == PassOutcome::Halted)
        .map(|(feed, _)| {
            format!(
                "{}: {}",
                feed.vendor,
                feed.last_error.as_deref().unwrap_or("terminal refusal")
            )
        })
        .collect();
    if !halted.is_empty() {
        return format!(
            "INCOMPLETE after {} pass(es); {landed_said} bar(s) added to the store census. \
             {} pass(es) were retried after a failure. Halted feed(s): {} \
             Full basket coverage has not been verified.{}",
            progress.passes,
            progress.retries,
            halted.join("; "),
            if progress.stopping {
                " The operator also pressed stop."
            } else if progress.passes >= MAX_PASSES {
                " The remaining work reached the pass ceiling."
            } else {
                ""
            }
        );
    }
    let mut summary = summary_of(progress.passes, progress.retries, landed, progress.stopping);
    summary.push_str(&standing_note(progress));
    summary
}

/// Records that one feed's chain stopped abnormally, on the feed it killed.
///
/// # The fingerprint this exists to name
///
/// A panicking task never reaches the code that clears its own fields, so it
/// leaves `doing` frozen on the leg it was running, `finished` still false and
/// `last_error` still null — measured 2026-08-20, alongside no journal record
/// and no socket, indefinitely. A feed that has STOPPED is then
/// indistinguishable from one working slowly, which is the confusion this
/// module exists to remove.
///
/// The panic itself goes to standard error through the panic hook and never
/// reaches `telemetry`, so this is the only surface that can say it happened.
fn note_dead_chain(
    site: &Site,
    run: u64,
    nth: usize,
    attempted: usize,
    dead: &tokio::task::JoinError,
) {
    with_progress(site, run, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
            // Labels may be empty. An explicit attempt count, not display
            // text, distinguishes a panicked request from an unattempted leg.
            let interrupted = u32::from(attempted > feed.legs_done as usize);
            feed.skipped = feed
                .legs
                .saturating_sub(feed.legs_done)
                .saturating_sub(interrupted);
            feed.finished = true;
            feed.doing = String::new();
            if feed.last_error.is_none() {
                feed.last_error = Some(format!(
                    "this feed's chain stopped abnormally and did not finish its \
                     legs ({dead}). The reason is on the server's standard error, \
                     which is the only place a panic is written; nothing already \
                     stored is affected, and the next pass asks for what is owed."
                ));
            }
        }
    });
}

/// The bars-added figure in words: the number, or `an unknown number of`
/// when the census total did not fit a `u64` (CE-74, D-1776).
fn landed_words(landed: Option<u64>) -> String {
    landed.map_or_else(
        || "an unknown number of".to_owned(),
        |rows| rows.to_string(),
    )
}

/// Describe an operator stop, pass ceiling, or idle stop. Store growth and
/// receipt verdicts cannot establish coverage of every requested instrument.
#[must_use]
pub fn summary_of(passes: u32, retries: u32, landed: Option<u64>, stopped: bool) -> String {
    let landed = landed_words(landed);
    // NOT `retried`: clippy denies a binding whose name is one letter from
    // `retries` beside it, and it is right — the two mean different things.
    let note = if retries > 0 {
        format!(" {retries} pass(es) were retried after a failure.")
    } else {
        String::new()
    };
    if stopped {
        return format!(
            "Stopped after {passes} pass(es); {landed} bar(s) added to the store census \
             before you pressed stop.{note} Full basket coverage has not been verified."
        );
    }
    if passes >= MAX_PASSES {
        return format!(
            "Reached the {MAX_PASSES}-pass ceiling with {landed} bar(s) added to the store census.{note} \
             This is a runaway stop and NOT a verdict on the window — press Pull \
             again to continue. Full basket coverage has not been verified."
        );
    }
    format!(
        "Idle retries stopped after {passes} pass(es); {landed} bar(s) added to the store census.{note} \
         The last {CLEAN_EMPTY_PASSES} passes reported no retryable failures and no row growth. \
         Empty responses or already-present bars can explain this. \
         Full basket coverage has not been verified."
    )
}

#[cfg(test)]
// THE SAME ALLOW EVERY TEST MODULE IN THIS CRATE CARRIES. A test that indexes a
// vector it just built, or expects a `Result` it just constructed, is asserting
// a shape rather than risking one -- and the panic IS the failure report.
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;

    /// What `encodeURIComponent` does to the characters this scheme depends on.
    ///
    /// A single pass, and that is the whole point: encoding `%` first and then
    /// `&` in a second pass would leave the `%` of `%26` un-escaped and the
    /// round trip would silently lose a byte. It is a test helper, not a URI
    /// encoder, and it handles exactly the characters a form body carries.
    fn enc(raw: &str) -> String {
        let mut out = String::new();
        for ch in raw.chars() {
            match ch {
                '%' => out.push_str("%25"),
                '&' => out.push_str("%26"),
                '=' => out.push_str("%3D"),
                '|' => out.push_str("%7C"),
                '+' => out.push_str("%2B"),
                c => out.push(c),
            }
        }
        out
    }

    /// One `leg` field, encoded exactly as the page encodes it, whose payload
    /// declares the vendor and rung its envelope names -- as the page's does.
    fn field(route: &str, vendor: &str, dir: &str, label: &str, body: &str) -> String {
        raw_field(
            route,
            vendor,
            dir,
            label,
            &format!("{}&{body}", declared(route, vendor, dir)),
        )
    }

    /// The payload fields that agree with an envelope, the way the page writes
    /// them.
    fn declared(route: &str, vendor: &str, dir: &str) -> String {
        if route == "/pull/fno" {
            let series = if dir == "futures" { "fut" } else { "opt" };
            format!("vendor={vendor}&series={series}")
        } else {
            format!("vendor={vendor}&granularity={dir}")
        }
    }

    /// One `leg` field whose payload is exactly `body`, agreeing or not.
    fn raw_field(route: &str, vendor: &str, dir: &str, label: &str, body: &str) -> String {
        let joined = format!("{route}|{vendor}|{dir}|{label}|{}", enc(body));
        format!("leg={}", enc(&joined))
    }

    /// An envelope that names one request while its payload asks for another
    /// refuses the whole run, on either half and on either route.
    ///
    /// The chain, the ladder order and the failed-daily gate all read the
    /// envelope; `pull_spot` and `pull_fno` run the payload. A `groww|1day`
    /// leg carrying `granularity=1min` and no `vendor` would have run a Dhan
    /// minute pull filed as a Groww day pass (P1-02-01, D-1765).
    #[test]
    fn a_leg_whose_payload_disagrees_with_its_envelope_refuses_the_run() {
        let refused = |leg: String, needle: &str| match legs_from(&leg) {
            Err(Refusal::Disagrees { leg, why }) => {
                assert!(why.contains(needle), "{why}");
                assert!(Refusal::Disagrees { leg, why }.why().contains(needle));
            }
            other => panic!("a disagreeing leg must be refused, got {other:?}"),
        };
        // The finding's own repro: no vendor means Dhan, 1min is not 1day.
        refused(
            raw_field(
                "/pull/spot",
                "groww",
                "1day",
                "x",
                "member=NIFTY&granularity=1min",
            ),
            "\"groww\"",
        );
        refused(
            raw_field(
                "/pull/spot",
                "groww",
                "1day",
                "x",
                "vendor=groww&granularity=1min",
            ),
            "\"1min\"",
        );
        // An absent granularity means one minute, not the envelope's day.
        refused(
            raw_field("/pull/spot", "dhan", "1day", "x", "vendor=dhan"),
            "granularity",
        );
        refused(
            raw_field(
                "/pull/fno",
                "dhan",
                "options",
                "x",
                "vendor=dhan&series=fut",
            ),
            "\"fut\"",
        );
        refused(
            raw_field("/pull/fno", "dhan", "futures", "x", "vendor=dhan"),
            "series",
        );
        refused(
            raw_field(
                "/pull/fno",
                "groww",
                "options",
                "x",
                "vendor=zerodha&series=opt",
            ),
            "\"zerodha\"",
        );
        // The same leg, agreeing, is read; the vendor matches case-blind the
        // way the route that runs it reads it.
        for agreeing in [
            raw_field(
                "/pull/spot",
                "groww",
                "1day",
                "x",
                "vendor=Groww&granularity=1DAY",
            ),
            raw_field("/pull/spot", "dhan", "1min", "x", "member=NIFTY"),
            raw_field(
                "/pull/fno",
                "dhan",
                "futures",
                "x",
                "vendor=dhan&series=fut",
            ),
            raw_field(
                "/pull/fno",
                "groww",
                "options",
                "x",
                "vendor=groww&series=opt",
            ),
        ] {
            assert_eq!(
                legs_from(&agreeing).map(|legs| legs.len()),
                Ok(1),
                "{agreeing}"
            );
        }
    }

    /// A form body survives the two encodings intact, INCLUDING the characters
    /// that would otherwise read as separators.
    ///
    /// This is the assertion the wire format exists for. A body carrying `&`,
    /// `=`, `|`, `+` and a literal `%` escape covers every character that either
    /// layer treats specially; if the two passes were not symmetric, one of them
    /// would come back changed and `pull_spot` would be handed a request the
    /// operator did not make.
    #[test]
    fn a_body_carrying_every_separator_survives_both_encodings() {
        let body = "member=BANKNIFTY&seg=futures%2Coptions&note=a|b+c&pct=100%";
        let body = format!("vendor=dhan&granularity=1day&{body}");
        let legs = legs_from(&raw_field(
            "/pull/spot",
            "dhan",
            "1day",
            "Spot · 1 day",
            &body,
        ))
        .expect("one well-formed leg");
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].body, body, "the body reaches the route unchanged");
        assert_eq!(legs[0].route, Route::Spot);
        assert_eq!(legs[0].vendor, "dhan");
        assert_eq!(legs[0].dir, "1day");
        assert_eq!(legs[0].label, "Spot · 1 day");
    }

    /// h-api-2, D-1512: a leg's payload is a form that reaches `pull_spot` and
    /// `pull_fno` without the D-1587 middleware, so a repeated single-value field in
    /// it refuses the WHOLE run, naming the key, while a repeated `member` in
    /// the same payload is a list and passes.
    #[test]
    fn a_leg_whose_payload_repeats_a_single_value_field_refuses_the_run() {
        let good = field(
            "/pull/spot",
            "dhan",
            "1day",
            "ok",
            "member=A&member=B&from=x",
        );
        let twice = field("/pull/spot", "dhan", "1day", "bad", "from=x&to=a&from=y");
        assert_eq!(legs_from(&good).expect("a list is not a repeat").len(), 1);
        match legs_from(&format!("{good}&{twice}")) {
            Err(Refusal::Malformed(why)) => {
                assert!(
                    why.ends_with("(its form names \"from\" more than once)"),
                    "{why}"
                );
                assert!(
                    Refusal::Malformed(why)
                        .why()
                        .contains("NOTHING was started")
                );
            }
            other => panic!("a repeated payload field must refuse the run: {other:?}"),
        }
    }

    /// DB-B32: the document names the run's own start and what each leg
    /// asked, read from the leg's own form, so a reloaded or second tab adopts
    /// them instead of its own clock, feed and selection.
    #[test]
    fn the_document_carries_the_runs_own_start_and_each_legs_ask() {
        let form = [
            field(
                "/pull/spot",
                "zerodha",
                "1min",
                "Spot · 1 minute",
                "target=n50&from=2018-01-01&to=2026-09-30&member=NIFTY&member=BANKNIFTY",
            ),
            field(
                "/pull/spot",
                "zerodha",
                "1day",
                "Spot · 1 day",
                "target=n50&from=2018-01-01&to=2026-09-30",
            ),
        ]
        .join("&");
        let legs = legs_from(&form).expect("two legs");
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_791_600_000_123);
        let progress = Progress::claimed_for(&legs, at);
        assert!(progress.running(), "a claimed document is a running one");
        assert_eq!(progress.started_at_ms, Some(1_791_600_000_123));
        assert_eq!(progress.asked.len(), 2);
        assert_eq!(progress.asked[0].members, 2);
        assert_eq!(
            progress.asked[1].members, 0,
            "no member field is every member"
        );
        let doc: serde_json::Value = serde_json::from_str(&progress.json()).expect("json");
        assert_eq!(doc["startedAtMs"], 1_791_600_000_123_u64);
        assert_eq!(doc["asked"][0]["vendor"], "zerodha");
        assert_eq!(doc["asked"][0]["dir"], "1min");
        assert_eq!(doc["asked"][0]["route"], "/pull/spot");
        assert_eq!(doc["asked"][0]["target"], "n50");
        assert_eq!(doc["asked"][0]["from"], "2018-01-01");
        assert_eq!(doc["asked"][0]["to"], "2026-09-30");
        assert_eq!(doc["asked"][0]["label"], "Spot · 1 minute");
        assert_eq!(doc["asked"][1]["dir"], "1day");
        // THE NEVER-RAN DOCUMENT STATES NO START AND NO ASK.
        let idle: serde_json::Value =
            serde_json::from_str(&Progress::default().json()).expect("json");
        assert!(idle["startedAtMs"].is_null());
        assert_eq!(idle["asked"], serde_json::json!([]));
    }

    /// Several legs, and the order they were sent in is the order they arrive.
    #[test]
    fn every_leg_in_the_form_is_read_and_none_is_dropped() {
        let form = [
            field("/pull/spot", "dhan", "1day", "d", "a=1"),
            field("/pull/fno", "groww", "options", "o", "b=2"),
            field("/pull/spot", "zerodha", "1min", "m", "c=3"),
        ]
        .join("&");
        let legs = legs_from(&form).expect("three legs");
        assert_eq!(legs.len(), 3);
        assert_eq!(legs[1].route, Route::Fno);
        assert_eq!(legs[2].vendor, "zerodha");
    }

    /// A body with no `leg` at all is refused rather than run as an empty set.
    ///
    /// An empty run would report a finished window having asked for nothing,
    /// which is the report-with-nothing-behind-it `CLAUDE.md` §4 bans.
    #[test]
    fn a_form_with_no_legs_is_refused_by_name() {
        assert_eq!(legs_from("feed=dhan&tf=1day"), Err(Refusal::NothingAsked));
        assert_eq!(legs_from(""), Err(Refusal::NothingAsked));
        assert!(!Refusal::NothingAsked.why().is_empty());
    }

    /// A leg missing a part refuses the WHOLE run, and names the leg.
    ///
    /// Not "skip that one and run the rest": a run that quietly dropped the leg
    /// it could not read would be reported to the operator as a completed
    /// window over an instrument it never asked for. This is the same
    /// abandon-the-rest class D-0220 fixed in four places.
    #[test]
    fn a_leg_that_cannot_be_read_refuses_the_whole_run() {
        let short = format!("leg={}", enc("/pull/spot|dhan|1day"));
        match legs_from(&short) {
            Err(Refusal::Malformed(named)) => {
                assert!(
                    named.contains("dhan"),
                    "the refusal quotes the leg: {named}"
                );
            }
            other => panic!("a four-part leg must be refused, got {other:?}"),
        }

        let good = field("/pull/spot", "dhan", "1day", "d", "a=1");
        let mixed = format!("{good}&{short}");
        assert!(
            matches!(legs_from(&mixed), Err(Refusal::Malformed(_))),
            "one bad leg refuses the run even when the others are fine"
        );
    }

    /// A route this build does not serve is named, never mapped onto a default.
    #[test]
    fn an_unknown_route_is_refused_rather_than_sent_to_spot() {
        let hostile = format!("leg={}", enc("/pull/everything|dhan|1day|d|a%3D1"));
        assert!(matches!(legs_from(&hostile), Err(Refusal::Malformed(_))));
        assert_eq!(Route::parse("/pull/spot"), Some(Route::Spot));
        assert_eq!(Route::parse("/pull/fno"), Some(Route::Fno));
        assert_eq!(Route::parse(""), None);
        assert_eq!(Route::Spot.path(), "/pull/spot");
        assert_eq!(Route::Fno.path(), "/pull/fno");
    }

    /// A vendor this build does not have refuses the whole run, so the groups
    /// `by_feed` builds can never outnumber the feeds.
    ///
    /// `by_feed` finds a leg's group by scanning the groups already built, and
    /// `conduct` spawns one chain per group. Both costs are bounded by the
    /// group count, and the group count is bounded by the feed count ONLY if
    /// every leg names a real feed. Before D-0906 `legs_from` copied the vendor
    /// unchecked, so a form of invented vendor names grew one group and one
    /// chain per distinct name. W1-api3-3.
    /// P3-01-01, D-1769: one leg past the bound is refused by name, and the
    /// widest leg the page can write fits the field bound.
    #[test]
    fn a_run_past_the_leg_bound_is_refused_by_name_and_the_widest_leg_fits() {
        let one = field("/pull/spot", "dhan", "1day", "d", "a=1");
        let full = vec![one.clone(); MAX_RUN_LEGS].join("&");
        assert_eq!(
            legs_from(&full).map(|legs| legs.len()).ok(),
            Some(MAX_RUN_LEGS)
        );
        let over = vec![one; MAX_RUN_LEGS + 1].join("&");
        match legs_from(&over) {
            Err(Refusal::TooManyLegs(count)) => {
                assert_eq!(count, MAX_RUN_LEGS + 1);
                let why = Refusal::TooManyLegs(count).why();
                assert!(why.contains(&MAX_RUN_LEGS.to_string()), "{why}");
            }
            other => panic!("one leg past the bound must be named, got {other:?}"),
        }
        // The page's own shape: a member form of `MAX_MEMBERS` symbols that
        // each need escaping, encoded once more for the payload and once
        // more for the field.
        let encode = |text: &str| -> String {
            text.bytes()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                        char::from(b).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect()
        };
        let symbol = "&".repeat(brutex_core::symbol::SYMBOL_CAPACITY);
        let members =
            vec![format!("member={}", encode(&symbol)); crate::ingest::MAX_MEMBERS].join("&");
        let form = format!("target=nifty50&vendor=dhan&{members}");
        assert!(
            form.len() <= crate::ingest::MAX_MEMBER_FORM_BYTES,
            "{}",
            form.len()
        );
        let leg = format!(
            "leg={}",
            encode(&["/pull/spot", "dhan", "1day", "label", &encode(&form)].join("|"))
        );
        assert!(
            leg.len() <= MAX_LEG_FIELD_BYTES,
            "{} > {MAX_LEG_FIELD_BYTES}",
            leg.len()
        );
        const { assert!(MAX_RUN_FORM_BYTES > MAX_RUN_LEGS * MAX_LEG_FIELD_BYTES) };
    }

    /// The figures `docs/06-limits.md` states for the form bounds, from the
    /// constants: 168,192 for a member form, 881,924 for one leg field and
    /// 27,347,836 for a run form (8,192 + 31 × 881,924). The prose said "about
    /// 26.5 MB", which is neither 27.35 MB nor 26.08 MiB. P17-19, D-1967.
    #[test]
    fn the_run_form_bound_is_the_figure_the_limits_document_states() {
        assert_eq!(crate::ingest::MAX_MEMBER_FORM_BYTES, 168_192);
        assert_eq!(MAX_LEG_FIELD_BYTES, 881_924);
        assert_eq!(MAX_RUN_LEGS, 30);
        assert_eq!(MAX_RUN_FORM_BYTES, 27_347_836);
        let limits = include_str!("../../../docs/06-limits.md");
        assert!(limits.contains("read up to 27,347,836 bytes of form"));
        assert!(!limits.contains("about 26.5 MB"));
    }

    #[test]
    fn a_leg_naming_no_feed_refuses_the_run_so_groups_never_outnumber_feeds() {
        let invented: Vec<String> = (0..=pull::vendor::FEED_COUNT)
            .map(|n| field("/pull/spot", &format!("nofeed{n}"), "1day", "x", "a=1"))
            .collect();
        match legs_from(&invented.join("&")) {
            Err(Refusal::Malformed(named)) => assert!(
                named.contains("nofeed0"),
                "the refusal quotes the leg that named no feed: {named}"
            ),
            other => panic!("a leg naming no feed must be refused, got {other:?}"),
        }
        // One invented vendor among real ones still refuses the whole run.
        let mixed = [
            field("/pull/spot", "dhan", "1day", "d", "a=1"),
            field("/pull/spot", "Dhan", "1day", "d", "a=1"),
        ]
        .join("&");
        assert!(
            matches!(legs_from(&mixed), Err(Refusal::Malformed(_))),
            "a vendor is matched on its exact wire spelling"
        );
        // Every feed this build has is accepted, and they group into exactly
        // one chain each -- the bound the cost argument above names.
        let every: Vec<String> = pull::vendor::Feed::ALL
            .iter()
            .flat_map(|feed| {
                [
                    field("/pull/spot", feed.wire(), "1day", "d", "a=1"),
                    field("/pull/spot", feed.wire(), "1min", "m", "b=2"),
                ]
            })
            .collect();
        let groups = by_feed(legs_from(&every.join("&")).expect("every feed is a vendor"));
        assert_eq!(groups.len(), pull::vendor::FEED_COUNT);
        assert!(groups.iter().all(|(_, legs)| legs.len() == 2));
    }

    /// The ladder, and the one position an unranked rung must never take.
    ///
    /// `position` answers `None` for a rung nobody listed, and sorting those
    /// FIRST is exactly what breaks `pull::fold`: the day pass for a month must
    /// land before the minute pass for it.
    #[test]
    fn an_unranked_rung_sorts_last_and_never_first() {
        assert!(ladder_rank("1day") < ladder_rank("1min"));
        assert!(ladder_rank("1min") < ladder_rank("futures"));
        assert!(ladder_rank("futures") < ladder_rank("options"));
        assert!(
            ladder_rank("options") < ladder_rank("a-rung-nobody-listed"),
            "an unknown rung goes last"
        );
    }

    /// Feeds keep the order they were ticked; legs inside one are put on the
    /// ladder.
    #[test]
    fn legs_group_by_feed_in_tick_order_and_sort_onto_the_ladder() {
        let form = [
            field("/pull/fno", "dhan", "options", "d-opt", "a=1"),
            field("/pull/spot", "groww", "1min", "g-min", "b=2"),
            field("/pull/spot", "dhan", "1day", "d-day", "c=3"),
            field("/pull/spot", "groww", "1day", "g-day", "d=4"),
        ]
        .join("&");
        let groups = by_feed(legs_from(&form).expect("four legs"));

        assert_eq!(groups.len(), 2, "two vendors, two chains");
        assert_eq!(groups[0].0, "dhan", "dhan was ticked first and stays first");
        assert_eq!(groups[1].0, "groww");
        assert_eq!(
            groups[0]
                .1
                .iter()
                .map(|l| l.label.as_str())
                .collect::<Vec<_>>(),
            ["d-day", "d-opt"],
            "spot before the derivatives it feeds"
        );
        assert_eq!(
            groups[1]
                .1
                .iter()
                .map(|l| l.label.as_str())
                .collect::<Vec<_>>(),
            ["g-day", "g-min"],
            "the day rung before the minute rung"
        );
    }

    /// Two legs on ONE rung keep the order the operator ticked them in.
    ///
    /// `sort_by_key` is stable and this pins that it stays so: a sort that
    /// reordered equal rungs would make the run non-reproducible for no reason.
    #[test]
    fn two_legs_on_one_rung_keep_the_order_they_were_ticked_in() {
        let form = [
            field("/pull/spot", "dhan", "1day", "first", "a=1"),
            field("/pull/spot", "dhan", "1day", "second", "b=2"),
            field("/pull/spot", "dhan", "1day", "third", "c=3"),
        ]
        .join("&");
        let groups = by_feed(legs_from(&form).expect("three legs"));
        assert_eq!(
            groups[0]
                .1
                .iter()
                .map(|l| l.label.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "third"]
        );
    }

    /// A fresh site's document says `running:false`, not nothing.
    ///
    /// The page asks this on load. A shape that differed before the first run
    /// would make "never pressed" and "server has died" read the same.
    #[test]
    fn a_run_that_never_started_still_answers_a_whole_document() {
        let doc = Progress::default().json();
        assert!(doc.contains("\"running\":false"), "{doc}");
        assert!(doc.contains("\"feeds\":[]"), "{doc}");
        assert!(doc.contains("\"finished\":null"), "{doc}");
        assert!(
            !Progress::default().running(),
            "a site that was never pressed is not running"
        );
        assert!(
            Progress::claimed().running(),
            "and a claimed slot is — otherwise the refusal below could never fire"
        );
    }

    /// `running` is `finished.is_none()` and nothing else.
    #[test]
    fn a_run_is_in_flight_exactly_while_it_has_no_summary() {
        let mut progress = Progress::claimed();
        assert!(
            progress.running(),
            "claimed and no summary yet means still going"
        );
        progress.finished = Some("done".to_owned());
        assert!(!progress.running(), "a summary is the end");
        assert!(progress.json().contains("\"running\":false"));
    }

    /// CE-74: a census total past `u64::MAX` is unknown, not a panic, and the
    /// page is told so.
    #[test]
    fn a_row_total_that_does_not_fit_is_unknown_not_a_panic() {
        assert_eq!(rows_total([u64::MAX, 1].into_iter()), None);
        assert_eq!(rows_total([u64::MAX - 1, 1].into_iter()), Some(u64::MAX));
        assert_eq!(rows_total(std::iter::empty()), Some(0));
        let unknown = Progress {
            rows_at_start: Some(5),
            rows_now: None,
            started: true,
            ..Progress::default()
        };
        let json = unknown.json();
        assert!(json.contains(r#""rowsAtStart":5,"rowsNow":null"#), "{json}");
        assert!(summary_of(2, 0, None, false).contains("an unknown number of bar(s)"));
    }

    /// Every field the page reads is in the document, with the name it reads.
    ///
    /// Hand-written JSON has no compiler checking the key names against the
    /// reader, so this is that check.
    #[test]
    fn the_document_carries_every_field_the_page_reads() {
        let progress = Progress {
            passes: 7,
            retries: 2,
            rows_at_start: Some(10),
            rows_now: Some(99),
            stopping: true,
            finished: None,
            started: true,
            generation: 0,
            started_at_ms: Some(1_791_000_000_000),
            asked: Vec::new(),
            feeds: vec![FeedReport {
                vendor: "dhan".to_owned(),
                legs: 4,
                legs_done: 3,
                doing: "Spot · 1 day".to_owned(),
                retries: 1,
                last_error: Some("a leg failed".to_owned()),
                finished: false,
                credential_dead: false,
                skipped: 0,
                standing: 0,
                store_unavailable: false,
            }],
            stop: None,
        };
        let doc = progress.json();
        for key in [
            "\"running\":true",
            "\"passes\":7",
            "\"retries\":2",
            "\"rowsAtStart\":10",
            "\"rowsNow\":99",
            "\"stopping\":true",
            "\"vendor\":\"dhan\"",
            "\"legs\":4",
            "\"legsDone\":3",
            "\"doing\":\"Spot · 1 day\"",
            "\"lastError\":\"a leg failed\"",
            "\"finished\":false",
        ] {
            assert!(doc.contains(key), "missing {key} in {doc}");
        }
    }

    /// Two feeds are two objects, separated.
    #[test]
    fn two_feeds_render_as_two_comma_separated_objects() {
        let progress = Progress {
            feeds: vec![
                FeedReport {
                    vendor: "dhan".to_owned(),
                    ..FeedReport::default()
                },
                FeedReport {
                    vendor: "groww".to_owned(),
                    ..FeedReport::default()
                },
            ],
            ..Progress::default()
        };
        let doc = progress.json();
        assert!(doc.contains("\"dhan\""), "{doc}");
        assert!(doc.contains("\"groww\""), "{doc}");
        assert!(doc.contains("},{"), "two objects are separated: {doc}");
    }

    /// A vendor message cannot break the document.
    ///
    /// A status page that will not parse is indistinguishable from a server
    /// that has died -- which is exactly the confusion this module removes, so
    /// a quote or a control character in a vendor's words must not cause it.
    #[test]
    fn a_reason_carrying_quotes_and_control_bytes_still_parses() {
        assert_eq!(quote_for_json("plain"), "\"plain\"");
        assert_eq!(quote_for_json("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quote_for_json("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(quote_for_json("a\nb\rc\td"), "\"a\\nb\\rc\\td\"");
        assert_eq!(
            quote_for_json("bell\u{7}"),
            "\"bell\\u0007\"",
            "a control byte is escaped, never dropped"
        );
    }

    /// The three endings are distinguishable, which is the whole value of the
    /// summary.
    ///
    /// A run that hit the pass ceiling has NOT proved the window empty, and
    /// saying so in the same words as one that did would tell the operator the
    /// backfill was complete when it was cut short.
    #[test]
    fn the_ceiling_stop_is_not_worded_as_a_finished_window() {
        let finished = summary_of(3, 0, Some(500), false);
        let ceiling = summary_of(MAX_PASSES, 4, Some(500), false);
        let stopped = summary_of(9, 1, Some(500), true);

        assert!(finished.contains("Idle retries stopped"), "{finished}");
        assert!(finished.contains("Full basket coverage has not been verified"));
        assert!(!finished.contains("runaway"), "{finished}");
        assert!(ceiling.contains("runaway stop"), "{ceiling}");
        assert!(!ceiling.contains("no more data"), "{ceiling}");
        assert!(stopped.contains("pressed stop"), "{stopped}");

        assert!(
            !finished.contains("were retried"),
            "no retries, no retry sentence: {finished}"
        );
        assert!(ceiling.contains("4 pass(es) were retried"), "{ceiling}");
    }

    /// Every refusal says something, and they are not the same something.
    #[test]
    fn each_refusal_names_its_own_cause() {
        let nothing = Refusal::NothingAsked.why();
        let malformed = Refusal::Malformed("bad".to_owned()).why();
        let running = Refusal::AlreadyRunning.why();
        assert!(malformed.contains("bad"), "it quotes the leg: {malformed}");
        assert!(running.contains("already in flight"), "{running}");
        assert_ne!(nothing, running);
        assert_ne!(nothing, malformed);
    }

    /// A scratch store, and a masters directory holding one index.
    ///
    /// Local to this module rather than borrowed from `server`'s test helpers,
    /// which are private to that module. The shape is `audit_json`'s, for the
    /// same reason: a `Site` needs both a universe and a store root, and neither
    /// may be the operator's.
    fn site(name: &str) -> Loaded {
        let store = crate::scratch::path(&format!("pullrun-{name}"));
        let _ = std::fs::remove_dir_all(&store);
        std::fs::create_dir_all(store.join("manifest")).expect("mkdir");

        let masters = crate::scratch::path(&format!("pullrun-masters-{name}"));
        std::fs::create_dir_all(&masters).expect("mkdir");
        std::fs::write(
            masters.join("groww_instruments.csv"),
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n",
        )
        .expect("write");

        // `Site::load` sets `Broker::Refused`, which is what every test wants:
        // no socket is opened, so driving `conduct` here contacts no vendor.
        // Wrapped because `conduct` takes a `Loaded` — the shared handle the
        // route hands it — and every spawned chain clones it.
        Loaded::new(Site::load(&masters, &store))
    }

    /// Claims the run slot the way `POST /pull/run` does, so a test can drive
    /// [`conduct`] directly.
    ///
    /// `conduct` edits through [`with_progress`], which is a no-op when the slot
    /// is empty — so a test that skipped this would exercise the loop and
    /// observe nothing, which is the shape of a test that asserts nothing.
    fn claim(site: &Site) -> u64 {
        let mut held = site
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let claimed = Progress::claimed();
        let run = claimed.generation;
        *held = Some(claimed);
        run
    }

    /// The generation of whatever claim the slot holds, `0` when it holds none:
    /// the number a run driven by a test addresses its edits to.
    fn current(site: &Site) -> u64 {
        site.run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map_or(0, |progress| progress.generation)
    }

    /// Edits whatever document the slot holds, as an operator route does: a
    /// stop or a test's setup is addressed to the slot, not to one run.
    fn edit_slot<F: FnOnce(&mut Progress)>(site: &Site, edit: F) {
        if let Some(progress) = site
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_mut()
        {
            edit(progress);
        }
    }

    /// Reads the live document back, as the page's poll would.
    fn observed(site: &Site) -> Progress {
        let held = site
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.clone().unwrap_or_default()
    }

    fn leg(vendor: &str, dir: &str) -> Leg {
        Leg {
            route: Route::Spot,
            vendor: vendor.to_owned(),
            dir: dir.to_owned(),
            label: format!("{vendor} · {dir}"),
            body: String::new(),
        }
    }

    /// **`conduct` HAD ONE CALLER AND NO TEST. THIS IS THAT TEST.**
    ///
    /// The function an operator's press runs — the pass loop, the per-vendor
    /// spawn, the sequential legs, the ticker, the summary and the slot release
    /// — was never executed by the suite. Every property this module documents
    /// was held by reading. `docs/04-invariants.md` A-48 says so in its own
    /// words: no test observes two vendors on the wire at once.
    ///
    /// # What this drives, and why the stop path is the one that can be driven
    ///
    /// A pass whose legs FAIL sleeps [`RETRY_WAIT`] and goes again, without a
    /// ceiling short of [`MAX_PASSES`] — by design, and it makes a failing run
    /// untestable in wall-clock terms. The stop is the deterministic entry: it
    /// is checked at the top of the loop, so a run stopped before it starts
    /// exercises the whole scaffold and returns at once.
    ///
    /// It proves four things that were previously only argued:
    /// the groups are built from the legs; the ticker is spawned and aborted
    /// rather than leaked; the summary is written; and the slot is released so a
    /// second press is not refused forever by a run that has ended.
    #[tokio::test]
    async fn conduct_runs_the_scaffold_and_releases_the_slot_when_stopped() {
        let site = site("conductstop");
        claim(&site);
        edit_slot(&site, |progress| progress.stopping = true);

        conduct(
            Loaded::clone(&site),
            current(&site),
            vec![leg("dhan", "1day"), leg("groww", "1day")],
        )
        .await;

        let seen = observed(&site);
        assert_eq!(
            seen.feeds.len(),
            2,
            "the groups are built from the legs, one per vendor: {:?}",
            seen.feeds
        );
        assert!(
            seen.finished.is_some(),
            "a run that ends writes a summary — `finished: None` is the ONLY \
             reading of `a run is in flight`, and leaving it would refuse every \
             later press against a run that no longer exists"
        );
        assert!(
            !seen.running(),
            "and the slot is therefore free for the next press"
        );
        assert_eq!(seen.passes, 0, "stopped before the first pass ran");
    }

    /// **ONE PRESS, AND BARS ARE ON DISK — THE WHOLE PATH, NO VENDOR.**
    ///
    /// The proof that was missing, and the reason it was missing: every other
    /// route through [`conduct`] ends at a broker, and this repository's
    /// standing rule is that no live vendor request originates from a test.
    ///
    /// **An archive feed needs none.** `Transport::LocalArchive` is CSV files on
    /// disk — no socket, no token, no governor — so a leg naming one drives the
    /// identical path a broker leg takes: `conduct` → per-vendor spawn →
    /// sequential legs → `pull_spot` → `run_local` → `pull::ingest::from_dir` →
    /// the census, the store and the counter file.
    ///
    /// So this is the end-to-end assertion the suite never had: **press, and
    /// bars exist that did not exist before.** Not a receipt, not a status
    /// document, not a count the run reported about itself — a file on disk,
    /// read back through the store's own reader.
    ///
    /// # What it would catch
    ///
    /// Every wiring defect between the press and the disk, which is where this
    /// session's findings lived: a leg that never reaches its route, a route
    /// that reaches no ingest, an ingest that files under the wrong prefix, a
    /// pass loop that ends before the work is done, a summary written over a
    /// run still in flight.
    #[tokio::test]
    async fn one_press_over_an_archive_feed_puts_bars_on_disk() {
        use std::io::Write as _;

        // THE FIXTURE IS THE VENDOR'S OWN SHAPE, verbatim from
        // `GFDLNFO_TICK_01072025/Futures/-III/FINNIFTY-III.NFO.csv` — ten
        // fields, a header row, `DD/MM/YYYY`, and a stamp inside the session.
        let folder = crate::scratch::path("pullrun-archive-folder");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("mkdir");
        let mut f = std::fs::File::create(folder.join("NIFTY.NFO.csv")).expect("create");
        f.write_all(b"Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest\n")
            .expect("write");
        // A success fixture covers the whole session; two sparse ticks cannot
        // prove complete derived minute candles.
        for minute in 555..930 {
            writeln!(
                f,
                "NIFTY.NFO,01/07/2025,{:02}:{:02}:00,27674,0,0,0,0,65,65",
                minute / 60,
                minute % 60
            )
            .expect("write session");
        }

        let site = site("archivepress");
        let store = site.store_root.clone();

        // NOTHING IS HELD YET. Asserted rather than assumed: a test that finds
        // bars it did not put there proves nothing at all.
        let before = crate::census::read_all(&store)
            .iter()
            .filter_map(census::VendorCensus::counters)
            .map(|(_months, rows, _entries)| rows)
            .sum::<u64>();
        assert_eq!(before, 0, "the scratch store starts empty");

        claim(&site);
        // BOUNDED, AND THE BOUND IS NOT DECORATION.
        //
        // A transient failure sleeps `RETRY_WAIT` between passes, up to
        // `MAX_PASSES`, so the test must have its own short bound. Empty
        // receipts now end idle retries; the assertion on the store below
        // still rejects an empty fixture as proof that the archive was saved.
        //
        // The clean path takes a fraction of a second — no socket, no governor,
        // one folder — so anything approaching this bound is already the
        // failure. A test that hangs reports nothing; this one reports what it
        // was waiting for.
        let bound = core::time::Duration::from_secs(30);
        let ran = tokio::time::timeout(
            bound,
            conduct(
                Loaded::clone(&site),
                current(&site),
                // GDFL, BECAUSE THE FIXTURE ABOVE IS A GDFL FILE. Its own
                // comment says so — `GFDLNFO_TICK_01072025/...`, ten columns —
                // and it was run under `TrueData`, whose F&O row is five wide.
                // The pair was incoherent and nothing noticed, because
                // `run_local` hardcoded `Columns::Gdfl` for every archive feed.
                // Now the shape comes from the feed's own layout, so the feed
                // has to be the one these bytes actually came from. D-0344.
                vec![Leg {
                    route: Route::Spot,
                    vendor: pull::vendor::Feed::Gdfl.wire().to_owned(),
                    dir: "1min".to_owned(),
                    label: "archive · 1 minute".to_owned(),
                    body: format!(
                        "target=swept&vendor={}&from=2025-07-01&to=2025-07-01&folder={}",
                        pull::vendor::Feed::Gdfl.wire(),
                        folder.display()
                    ),
                }],
            ),
        )
        .await;
        assert!(
            ran.is_ok(),
            "the press did not finish inside {bound:?}. On this path that means \
             the leg FAILED and the pass loop is retrying it, potentially for \
             MAX_PASSES x RETRY_WAIT. Read the feed's `lastError`: {:?}",
            observed(&site).feeds
        );

        // THE STORE ITSELF, not the run's opinion of the store. `rows_now`
        // re-reads the manifests, which is the only reading of progress that
        // cannot be fooled by a receipt.
        let after = crate::census::read_all(&store)
            .iter()
            .filter_map(census::VendorCensus::counters)
            .map(|(_months, rows, _entries)| rows)
            .sum::<u64>();

        let seen = observed(&site);
        assert!(
            after > before,
            "one press stored nothing. The whole path from the leg to the disk \
             is what this asserts, and every wiring defect between them lands \
             here: {after} row(s) after, {before} before. Run summary: {:?}, \
             feeds: {:?}",
            seen.finished,
            seen.feeds
        );
        assert!(
            seen.finished.is_some(),
            "and the run ended rather than being left in flight forever"
        );
        assert_eq!(
            seen.feeds.len(),
            1,
            "one vendor was asked, so one chain was built: {:?}",
            seen.feeds
        );
    }

    /// **A PRESS IN FLIGHT STANDS THE AUTOPILOT OFF, BETWEEN LEGS TOO.**
    ///
    /// The autopilot already stood off for a SEAT, and a seat is per LEG.
    /// `pull_spot` takes its feed's seat and drops it when the leg returns,
    /// while a press is many legs across many passes — so between any two legs
    /// the mask reads zero, and `take_every_seat` is a
    /// `compare_exchange(0, ALL)`. The autopilot won that gap and held every
    /// feed for a whole month's pass; the operator's next leg 409'd, `conduct`
    /// slept `RETRY_WAIT` and tried again, and two drivers spent one shared
    /// token's quota against each other for as long as both kept going.
    ///
    /// `Progress::running` is the press-shaped fact — claimed before the first
    /// leg, released after the summary — so it covers the gaps a seat cannot.
    /// This drives the predicate the standoff reads, because the standoff itself
    /// lives in a tick that takes minutes and opens sockets.
    #[test]
    fn a_press_reads_as_running_from_its_claim_until_its_summary() {
        let site = site("pressflag");

        // BEFORE THE CLAIM. An unclaimed slot must not read as a press, or the
        // autopilot would stand off forever against a run nobody started —
        // which is the defect `Progress::started` was added to fix, reached
        // from the other side.
        assert!(
            !observed(&site).running(),
            "a site with no run is not pressing"
        );

        claim(&site);
        assert!(
            observed(&site).running(),
            "claimed and unfinished IS the window the autopilot must stand off \
             for — it spans the gaps between legs, which is the whole point"
        );

        // AND THE SUMMARY ENDS IT. A press that finished must not keep the
        // backfill standing off, or one hand-made pull would silence the
        // autopilot for the life of the process.
        edit_slot(&site, |progress| {
            progress.finished = Some("done".to_owned());
        });
        assert!(
            !observed(&site).running(),
            "the summary releases the standoff"
        );
    }

    /// **A RUN'S FINISHER NEVER FINISHES THE NEXT RUN.** D-2760, runs-1.
    ///
    /// The production order, replayed step by step with no clock: run A writes
    /// its summary, which frees the slot; press B is admitted and installs its
    /// own claim; only then does A's `Finisher` drop. Before the generation,
    /// that drop saw B's `finished: None`, stamped "ended abnormally" on a live
    /// run and freed the slot for a third press over the same store.
    #[test]
    fn a_finisher_dropped_after_the_next_claim_leaves_that_run_running() {
        let site = site("finishernext");

        // Its own run, unfinished: the Finisher DOES stamp it. The abnormal-end
        // guard still works; it is only addressed now.
        let lost = claim(&site);
        drop(Finisher {
            site: Loaded::clone(&site),
            run: lost,
        });
        let stamped = observed(&site);
        assert!(!stamped.running(), "an abnormal end still frees its slot");
        assert!(
            stamped
                .finished
                .as_deref()
                .is_some_and(|why| why.contains("stopped abnormally")),
            "{stamped:?}"
        );

        let first = claim(&site);
        let finisher = Finisher {
            site: Loaded::clone(&site),
            run: first,
        };
        with_progress(&site, first, |progress| {
            progress.finished = Some("A's own summary".to_owned());
        });
        let second = claim(&site);
        assert_ne!(first, second, "two claims never share a generation");
        drop(finisher);

        let seen = observed(&site);
        assert_eq!(seen.generation, second);
        assert!(
            seen.running(),
            "B is live, so its slot must still read as running: {seen:?}"
        );
        assert_eq!(seen.finished, None, "A's Finisher wrote into B");
    }

    /// **EVERY EDIT IS ADDRESSED.** A late write from an ended run (the ticker
    /// poll that was mid-flight at `abort`, a detached chain) lands nowhere once
    /// the slot holds a later claim, and that run reads itself as stopped.
    #[test]
    fn edits_from_an_ended_run_never_land_on_the_next_claim() {
        assert_ne!(
            Progress::claimed().generation,
            0,
            "zero is the never-claimed Default and no claim may reuse it"
        );
        let site = site("latetick");
        let first = claim(&site);
        edit_slot(&site, |progress| {
            progress.feeds = vec![FeedReport {
                credential_dead: true,
                ..FeedReport::default()
            }];
        });
        assert_eq!(halted_feeds(&site, first), vec![true]);
        assert!(!stopping(&site, first), "its own live claim is not stopped");

        let second = claim(&site);
        with_progress(&site, first, |progress| progress.rows_now = Some(7));
        with_progress(&site, first, |progress| progress.passes = 3);
        let seen = observed(&site);
        assert_eq!(seen.rows_now, None, "A's late tick landed in B: {seen:?}");
        assert_eq!(seen.passes, 0, "A's late write landed in B: {seen:?}");
        assert!(
            stopping(&site, first),
            "a run that no longer owns the slot must stop pulling"
        );
        assert!(!stopping(&site, second));
        edit_slot(&site, |progress| {
            progress.feeds = vec![FeedReport {
                credential_dead: true,
                ..FeedReport::default()
            }];
        });
        assert!(
            halted_feeds(&site, first).is_empty(),
            "A must not read B's halted feeds as its own"
        );
        with_progress(&site, second, |progress| progress.rows_now = Some(9));
        assert_eq!(
            observed(&site).rows_now,
            Some(9),
            "B's own edits still land"
        );
    }

    /// **THE SKIP LIST IS READ FROM THE LIVE DOCUMENT, PER FEED.**
    ///
    /// [`halted_feeds`] is what the spawn loop consults, so this drives it
    /// directly rather than through [`conduct`] — and the reason is worth
    /// recording, because it is a trap this test was written INTO first.
    ///
    /// # `conduct` rebuilds `feeds` at entry, so a flag cannot be pre-set
    ///
    /// The first version of this test marked both feeds `credential_dead`
    /// BEFORE calling `conduct`, expecting the pass loop to skip them and finish
    /// in milliseconds. It hung. `conduct` assigns `progress.feeds` from
    /// `groups` at entry with `..FeedReport::default()`, which is
    /// `credential_dead: false` — so the marks were wiped, the legs ran against
    /// a refused broker, every pass recorded a failure, and the loop began
    /// sleeping [`RETRY_WAIT`] up to [`MAX_PASSES`] times. **Two hours of test.**
    ///
    /// That is a fact about the TEST and not a defect in the loop: in production
    /// the flag is set by `run_chain` DURING a pass, and `feeds` is never
    /// rebuilt after that initial assignment, so it survives into the next
    /// pass's read. Written down here because the shape is genuinely
    /// counter-intuitive and the next person to reach for that test will reach
    /// the same way.
    #[test]
    fn a_halted_feed_is_reported_to_the_spawn_loop_and_a_healthy_one_is_not() {
        let held = site("haltedlist");
        claim(&held);
        edit_slot(&held, |progress| {
            progress.feeds = vec![
                FeedReport {
                    vendor: "dhan".to_owned(),
                    credential_dead: true,
                    ..FeedReport::default()
                },
                FeedReport {
                    vendor: "groww".to_owned(),
                    credential_dead: false,
                    ..FeedReport::default()
                },
            ];
        });

        assert_eq!(
            halted_feeds(&held, current(&held)),
            vec![true, false],
            "the skip list is BY POSITION, because that is how the spawn loop \
             indexes `groups` — a list that lost the order would skip the wrong \
             vendor, which is worse than skipping none"
        );

        // AND AN EMPTY SLOT YIELDS AN EMPTY LIST rather than panicking or
        // claiming everything is halted. `conduct` reads this before it has
        // written anything, and a `true` here would skip every feed on the
        // first pass — a run that asks for nothing and reports finishing.
        let fresh = site("haltedlistempty");
        assert!(
            halted_feeds(&fresh, current(&fresh)).is_empty(),
            "no run claimed, nothing halted"
        );
    }

    fn receipt(status: u16, verdict: &str) -> (axum::http::StatusCode, String) {
        (
            axum::http::StatusCode::from_u16(status).expect("HTTP status"),
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict,
                reason: "synthetic coordinator receipt",
                good: verdict == "STORED",
                facts: &[],
                footnote: "No vendor was contacted.",
            }),
        )
    }

    async fn simulated<F, Fut>(name: &str, legs: Vec<Leg>, request: F) -> Progress
    where
        F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
        Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
    {
        let held = site(name);
        claim(&held);
        tokio::time::timeout(
            core::time::Duration::from_secs(5),
            conduct_with(Loaded::clone(&held), current(&held), legs, request),
        )
        .await
        .expect("the synthetic run must finish without retry sleeps");
        observed(&held)
    }

    #[test]
    fn fixed_refusals_and_transient_conflicts_have_different_dispositions() {
        for code in [404, 422] {
            let (status, html) = receipt(code, "NOT STARTED");
            assert_eq!(leg_outcome(status, &html), LegOutcome::Permanent, "{code}");
        }
        for code in [401, 403] {
            let (status, html) = receipt(code, "NOT STARTED");
            assert_eq!(leg_outcome(status, &html), LegOutcome::Credential, "{code}");
        }
        for code in [400, 408, 409, 429, 500, 502, 503, 504] {
            let (status, html) = receipt(code, "NOT STARTED");
            assert_eq!(leg_outcome(status, &html), LegOutcome::Retry, "{code}");
        }
        let (status, html) = receipt(400, "REFUSED");
        assert_eq!(leg_outcome(status, &html), LegOutcome::Permanent);
        let (status, html) = receipt(200, "FAILED");
        assert_eq!(leg_outcome(status, &html), LegOutcome::Retry);
        let (_, green) = receipt(200, "STORED");
        assert_eq!(
            leg_outcome(status, &format!("{html}{green}")),
            LegOutcome::Retry,
            "a later good badge must not override the first receipt verdict"
        );
        assert_eq!(leg_outcome(status, "not a receipt"), LegOutcome::Retry);
        let (_, empty) = receipt(200, "EMPTY");
        assert_eq!(leg_outcome(status, &empty), LegOutcome::Empty);
        assert_eq!(leg_outcome(status, &green), LegOutcome::Stored);
        assert_eq!(
            leg_outcome(status, &format!("{html}<p>access token expired</p>")),
            LegOutcome::Credential,
            "a failed HTTP 200 can still carry a credential failure"
        );
    }

    /// DB-B37: a leg refused at the door carries the receipt's own reason,
    /// never "the reason is in the audit journal", which the refusal never
    /// wrote; a receipt with no `Refused because` line keeps the old wording.
    #[test]
    fn a_door_refusal_carries_the_receipts_own_reason() {
        let page = crate::render::receipt_page(&crate::render::Receipt {
            scope: "Spot pull",
            verdict: "NOT STARTED",
            reason: "x",
            good: false,
            facts: &[(
                "Refused because",
                "another pull already holds Zerodha's seat & <says> \"why\"".to_owned(),
            )],
            footnote: "Nothing here was written to the store.",
        });
        assert_eq!(
            receipt_refusal(&page).as_deref(),
            Some("another pull already holds Zerodha's seat & <says> \"why\"")
        );
        assert_eq!(receipt_refusal("<p>no facts</p>"), None);
        let held = site("door-refusal");
        claim(&held);
        let run = current(&held);
        with_progress(&held, run, |progress| {
            progress.feeds = vec![FeedReport::default(), FeedReport::default()];
        });
        let leg = Leg {
            route: Route::Spot,
            vendor: "zerodha".to_owned(),
            dir: "1min".to_owned(),
            label: "Spot · 1 minute".to_owned(),
            body: String::new(),
        };
        note_leg_failure(
            &held,
            run,
            0,
            &leg,
            (axum::http::StatusCode::CONFLICT, &page),
            LegOutcome::Retry,
        );
        note_leg_failure(
            &held,
            run,
            1,
            &leg,
            (axum::http::StatusCode::BAD_GATEWAY, "<p>no facts</p>"),
            LegOutcome::Retry,
        );
        let seen = observed(&held);
        let said = seen.feeds[0].last_error.clone().expect("a reason");
        assert!(
            said.contains("answered HTTP 409: another pull already holds Zerodha's seat"),
            "{said}"
        );
        assert!(
            !said.contains("The reason is in the audit journal"),
            "{said}"
        );
        let old = seen.feeds[1].last_error.clone().expect("a reason");
        assert!(old.contains("The reason is in the audit journal"), "{old}");
        // THE DOOR REFUSAL, AND ONLY IT, SAYS NO RECORD WAS WRITTEN.
        assert!(said.contains("no journal record was written"), "{said}");
    }

    /// DB-B37, repair: a receipt that carries `Refused because` AND its own
    /// `Recorded` line (a blocked run, a mapping that reached nothing: both
    /// append a record before answering) is never said to have written none.
    /// The receipt's `Recorded` line is carried as it stands, whichever way
    /// the append went.
    #[test]
    fn a_journalled_refusal_is_never_said_to_have_no_record() {
        let receipt_with = |recorded: &str| {
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict: "NOT STARTED",
                reason: "x",
                good: false,
                facts: &[
                    (
                        "Refused because",
                        "Instrument mapping is not ready".to_owned(),
                    ),
                    ("Recorded", recorded.to_owned()),
                ],
                footnote: "Nothing here was written to the store.",
            })
        };
        let landed = receipt_with("yes — appended to /x/pull.journal");
        let lost = receipt_with("NO — this run is NOT in the journal. the disk is full");
        assert_eq!(
            receipt_fact(&landed, "Recorded").as_deref(),
            Some("yes — appended to /x/pull.journal")
        );
        assert_eq!(receipt_fact(&landed, "Stored"), None);
        assert_eq!(receipt_fact("<th>Recorded</th><td>", "Recorded"), None);
        assert_eq!(
            receipt_fact("<th>Recorded</th><td>  </td>", "Recorded"),
            None
        );
        let held = site("journalled-refusal");
        claim(&held);
        let run = current(&held);
        with_progress(&held, run, |progress| {
            progress.feeds = vec![FeedReport::default(), FeedReport::default()];
        });
        let leg = Leg {
            route: Route::Spot,
            vendor: "zerodha".to_owned(),
            dir: "1min".to_owned(),
            label: "Spot · 1 minute".to_owned(),
            body: String::new(),
        };
        for (nth, page) in [&landed, &lost].into_iter().enumerate() {
            note_leg_failure(
                &held,
                run,
                nth,
                &leg,
                (axum::http::StatusCode::BAD_REQUEST, page),
                LegOutcome::Retry,
            );
        }
        let seen = observed(&held);
        let said = seen.feeds[0].last_error.clone().expect("a reason");
        assert!(
            said.contains("answered HTTP 400: Instrument mapping is not ready"),
            "{said}"
        );
        assert!(
            said.contains("the receipt's Recorded line: yes — appended to /x/pull.journal"),
            "{said}"
        );
        assert!(!said.contains("no journal record was written"), "{said}");
        let failed = seen.feeds[1].last_error.clone().expect("a reason");
        assert!(
            failed.contains(
                "the receipt's Recorded line: NO — this run is NOT in the journal. the disk is full"
            ),
            "{failed}"
        );
        assert!(
            !failed.contains("no journal record was written"),
            "{failed}"
        );
    }

    #[tokio::test]
    async fn a_dead_feed_remains_incomplete_after_healthy_feeds_finish() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "dead-and-healthy",
            vec![
                leg("zerodha", "1day"),
                leg("zerodha", "1min"),
                leg("groww", "1day"),
                leg("groww", "1min"),
            ],
            move |_site, leg| {
                capture.lock().expect("calls").push(leg.label);
                std::future::ready(if leg.vendor == "zerodha" {
                    receipt(401, "NOT STARTED")
                } else {
                    receipt(200, "STORED")
                })
            },
        )
        .await;
        let dead = &progress.feeds[0];
        let healthy = &progress.feeds[1];
        assert!(dead.credential_dead);
        assert_eq!((dead.legs_done, dead.skipped, dead.retries), (1, 1, 0));
        assert!(!healthy.credential_dead);
        assert_eq!(
            (healthy.legs_done, healthy.skipped, healthy.retries),
            (2, 0, 0)
        );
        assert!(healthy.finished);
        assert_eq!(progress.passes, CLEAN_EMPTY_PASSES);
        assert_eq!(progress.retries, 0, "halting is not retrying");
        let calls = calls.lock().expect("calls");
        assert_eq!(calls.iter().filter(|s| s.starts_with("zerodha")).count(), 1);
        assert_eq!(calls.iter().filter(|s| s.starts_with("groww")).count(), 6);
        assert!(!progress.running());
        let summary = progress.finished.expect("terminal summary");
        assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
        assert!(summary.contains("CREDENTIAL"));
        assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
    }

    #[tokio::test]
    async fn permanent_422_stops_after_one_response_without_retrying() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "permanent-preflight",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |_site, _leg| {
                capture.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::future::ready(receipt(422, "NOT STARTED"))
            },
        )
        .await;
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(
            (progress.passes, progress.retries, progress.rows_now),
            (1, 0, Some(0))
        );
        let feed = &progress.feeds[0];
        assert_eq!((feed.legs_done, feed.skipped, feed.retries), (1, 1, 0));
        assert!(
            !feed.credential_dead,
            "preflight is not an expired credential"
        );
        assert!(feed.finished);
        let summary = progress.finished.expect("terminal summary");
        assert!(summary.contains("INCOMPLETE"));
        assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
    }

    #[tokio::test]
    async fn empty_receipts_end_idle_retries_without_claiming_coverage() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "empty-is-unverified",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |_site, _leg| {
                capture.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::future::ready(receipt(200, "EMPTY"))
            },
        )
        .await;
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 6);
        assert_eq!(
            (progress.passes, progress.retries, progress.rows_now),
            (3, 0, Some(0))
        );
        assert_eq!(progress.feeds[0].legs_done, 2);
        let summary = progress.finished.expect("idle summary");
        assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
        assert!(summary.contains("Full basket coverage has not been verified"));
        assert!(!summary.contains("no more data"));
    }

    #[tokio::test]
    async fn operator_stop_counts_the_returned_leg_and_leaves_the_rest_unattempted() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "stop-between-legs",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |site, _leg| {
                assert_eq!(
                    observed(&site).feeds[0].legs_done,
                    0,
                    "in-flight is not done"
                );
                capture.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                edit_slot(&site, |progress| progress.stopping = true);
                std::future::ready(receipt(200, "STORED"))
            },
        )
        .await;
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(
            (progress.feeds[0].legs_done, progress.feeds[0].skipped),
            (1, 1)
        );
        assert_eq!(progress.retries, 0);
        assert!(progress.finished.expect("stopped").contains("pressed stop"));
    }

    fn pass_fixture(name: &str, legs: Vec<Leg>) -> (Loaded, Vec<(String, Vec<Leg>)>) {
        let held = site(name);
        claim(&held);
        let groups = by_feed(legs);
        edit_slot(&held, |progress| {
            progress.feeds = groups
                .iter()
                .map(|(vendor, legs)| FeedReport {
                    vendor: vendor.clone(),
                    legs: u32::try_from(legs.len()).expect("fixture leg count"),
                    ..FeedReport::default()
                })
                .collect();
        });
        (held, groups)
    }

    /// Drive the production pass seam without the conductor's retry sleep.
    /// Requests are identified by their full leg, including distinct bodies
    /// with identical display labels. No assertion depends on feed interleaving.
    struct Passes {
        site: Loaded,
        groups: Vec<(String, Vec<Leg>)>,
        outcomes: Vec<PassOutcome>,
        checkpoints: Vec<Checkpoints>,
    }

    impl Passes {
        fn new(name: &str, legs: Vec<Leg>) -> Self {
            let (site, groups) = pass_fixture(name, legs);
            let outcomes = vec![PassOutcome::Clean; groups.len()];
            let checkpoints = checkpoints_for(&groups);
            Self {
                site,
                groups,
                outcomes,
                checkpoints,
            }
        }

        async fn run<F, Fut>(&mut self, request: F)
        where
            F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
            Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
        {
            run_pass(
                &self.site,
                current(&self.site),
                &self.groups,
                &mut self.outcomes,
                &self.checkpoints,
                &request,
            )
            .await;
        }

        async fn respond(&mut self, replies: &[(&str, u16, &str)]) -> Vec<Leg> {
            let replies: Vec<_> = replies
                .iter()
                .map(|(body, status, verdict)| (body.to_string(), *status, verdict.to_string()))
                .collect();
            let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let capture = std::sync::Arc::clone(&calls);
            self.run(move |_site, leg| {
                capture.lock().expect("calls").push(leg.clone());
                std::future::ready(
                    replies
                        .iter()
                        .find(|(body, _, _)| *body == leg.body)
                        .map_or_else(
                            || receipt(200, "STORED"),
                            |(_, code, verdict)| receipt(*code, verdict),
                        ),
                )
            })
            .await;
            std::mem::take(&mut *calls.lock().expect("calls"))
        }
    }

    fn named_leg(vendor: &str, dir: &str, body: &str) -> Leg {
        Leg {
            route: if matches!(dir, "futures" | "options") {
                Route::Fno
            } else {
                Route::Spot
            },
            body: body.to_owned(),
            ..leg(vendor, dir)
        }
    }

    #[tokio::test]
    async fn retry_passes_only_request_owed_legs_including_successes_after_a_failure() {
        let day = named_leg("zerodha", "1day", "daily");
        let owed = named_leg("zerodha", "1min", "owed");
        let clean = named_leg("zerodha", "1min", "already");
        let future = named_leg("zerodha", "futures", "future");
        let option = named_leg("zerodha", "options", "option");
        assert_eq!(owed.label, clean.label, "labels are not checkpoint keys");
        let mut passes = Passes::new(
            "checkpoint-non-prefix",
            vec![
                option.clone(),
                day.clone(),
                owed.clone(),
                clean.clone(),
                future.clone(),
            ],
        );
        for (index, code) in [429, 409, 503].into_iter().enumerate() {
            let calls = passes.respond(&[("owed", code, "NOT STARTED")]).await;
            assert_eq!(
                calls,
                if index == 0 {
                    vec![day.clone(), owed.clone(), clean.clone()]
                } else {
                    vec![owed.clone()]
                }
            );
            assert_eq!(passes.outcomes, [PassOutcome::Retry]);
            let progress = observed(&passes.site);
            let feed = &progress.feeds[0];
            assert_eq!(feed.legs_done as usize, calls.len());
            assert_eq!(feed.skipped as usize, 5 - calls.len());
            assert_eq!(feed.retries as usize, index);
            assert_eq!(progress.retries, feed.retries);
            assert!(feed.last_error.as_ref().unwrap().contains("HTTP 429"));
            assert!(feed.finished && feed.doing.is_empty());
        }
        assert_eq!(
            passes.respond(&[]).await,
            vec![owed.clone(), future, option],
            "both FNO legs wait for the outstanding spot response"
        );
        assert_eq!(passes.outcomes, [PassOutcome::Clean]);
        assert_eq!(observed(&passes.site).feeds[0].retries, 3);
        assert_eq!(
            passes.respond(&[("daily", 503, "NOT STARTED")]).await,
            vec![day],
            "a fresh full pass resets checkpoints and rechecks dependencies"
        );
        assert_eq!(observed(&passes.site).feeds[0].retries, 3);
        assert_eq!(passes.outcomes, [PassOutcome::Retry]);
    }

    #[tokio::test]
    async fn failed_daily_and_minute_rungs_defer_dependents_until_their_retry_succeeds() {
        let first = named_leg("zerodha", "1day", "first");
        let second = named_leg("zerodha", "1day", "second");
        let minute = named_leg("zerodha", "1min", "minute");
        let second_bars = named_leg("zerodha", "1s", "seconds");
        let option = named_leg("zerodha", "options", "option");
        let mut passes = Passes::new(
            "checkpoint-dependencies",
            vec![
                minute.clone(),
                first.clone(),
                second.clone(),
                second_bars.clone(),
                option.clone(),
            ],
        );
        assert_eq!(
            passes.respond(&[("first", 409, "NOT STARTED")]).await,
            vec![first.clone(), second],
            "another daily leg can succeed even though the first is owed"
        );
        let feed = &observed(&passes.site).feeds[0];
        assert_eq!((feed.legs_done, feed.skipped), (2, 3));
        assert_eq!(
            passes.respond(&[("minute", 502, "FAILED")]).await,
            vec![first, minute.clone()]
        );
        assert_eq!(passes.outcomes, [PassOutcome::Retry]);
        assert_eq!(passes.respond(&[]).await, vec![minute, second_bars, option]);
        let feed = &observed(&passes.site).feeds[0];
        assert_eq!((feed.legs_done, feed.skipped, feed.retries), (3, 2, 2));
        assert_eq!(passes.outcomes, [PassOutcome::Clean]);
    }

    #[tokio::test]
    async fn healthy_feeds_wait_until_all_retrying_feeds_recover_before_a_fresh_pass() {
        let zerodha = named_leg("zerodha", "1day", "z");
        let groww = named_leg("groww", "1day", "g");
        let dhan = named_leg("dhan", "1day", "d");
        let mut passes = Passes::new(
            "checkpoint-siblings",
            vec![zerodha.clone(), groww.clone(), dhan.clone()],
        );
        let mut calls = passes
            .respond(&[("z", 503, "FAILED"), ("g", 429, "FAILED")])
            .await;
        calls.sort_by(|a, b| a.body.cmp(&b.body));
        assert_eq!(calls, vec![dhan.clone(), groww.clone(), zerodha.clone()]);
        let mut calls = passes.respond(&[("g", 409, "NOT STARTED")]).await;
        calls.sort_by(|a, b| a.body.cmp(&b.body));
        assert_eq!(calls, vec![groww.clone(), zerodha.clone()]);
        let healthy = &observed(&passes.site).feeds[2];
        assert_eq!(
            (healthy.legs_done, healthy.skipped, healthy.retries),
            (0, 1, 0)
        );
        assert!(healthy.finished && healthy.last_error.is_none());
        assert_eq!(passes.respond(&[]).await, vec![groww.clone()]);
        let progress = observed(&passes.site);
        assert_eq!(progress.retries, 2, "passes, not the sum of feed retries");
        assert_eq!(
            progress.feeds.iter().map(|f| f.retries).collect::<Vec<_>>(),
            [1, 2, 0]
        );
        let mut calls = passes.respond(&[]).await;
        calls.sort_by(|a, b| a.body.cmp(&b.body));
        assert_eq!(calls, vec![dhan, groww, zerodha]);
        assert_eq!(passes.outcomes, [PassOutcome::Clean; 3]);
        assert_eq!(observed(&passes.site).retries, 2);
    }

    #[tokio::test]
    async fn terminal_refusals_after_checkpointed_progress_never_resurrect_a_feed() {
        for (code, verdict) in [(401, "NOT STARTED"), (422, "NOT STARTED"), (400, "REFUSED")] {
            let day = named_leg("zerodha", "1day", "day");
            let minute = named_leg("zerodha", "1min", "minute");
            let option = named_leg("zerodha", "options", "option");
            let mut passes = Passes::new(
                &format!("checkpoint-terminal-{code}"),
                vec![day.clone(), minute.clone(), option],
            );
            assert_eq!(
                passes.respond(&[("minute", 503, "FAILED")]).await,
                vec![day, minute.clone()]
            );
            assert_eq!(
                passes.respond(&[("minute", code, verdict)]).await,
                vec![minute]
            );
            for _ in 0..2 {
                assert!(passes.respond(&[]).await.is_empty());
            }
            assert_eq!(passes.outcomes, [PassOutcome::Halted]);
            let progress = observed(&passes.site);
            let feed = &progress.feeds[0];
            assert_eq!((feed.legs_done, feed.skipped, feed.retries), (1, 2, 1));
            assert_eq!(feed.credential_dead, code == 401);
            assert_eq!(progress.retries, 1);
            let summary = run_summary(&progress, &passes.outcomes, Some(0));
            assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
            assert!(!summary.contains("HTTP 503"));
        }
    }

    #[tokio::test]
    async fn a_stop_during_retry_counts_only_the_returned_request_and_preserves_checkpoints() {
        let day = named_leg("zerodha", "1day", "day");
        let minute = named_leg("zerodha", "1min", "minute");
        let option = named_leg("zerodha", "options", "option");
        let mut passes = Passes::new("checkpoint-stop", vec![day.clone(), minute.clone(), option]);
        assert_eq!(
            passes.respond(&[("minute", 503, "FAILED")]).await,
            vec![day, minute.clone()]
        );
        passes
            .run(move |site, requested| {
                assert_eq!(requested, minute);
                assert_eq!(observed(&site).feeds[0].legs_done, 0);
                edit_slot(&site, |p| p.stopping = true);
                std::future::ready(receipt(200, "STORED"))
            })
            .await;
        let progress = observed(&passes.site);
        let feed = &progress.feeds[0];
        assert_eq!((feed.legs_done, feed.skipped, feed.retries), (1, 2, 1));
        assert!(feed.finished && feed.doing.is_empty());
        assert!(run_summary(&progress, &passes.outcomes, Some(0)).contains("pressed stop"));
        assert!(passes.respond(&[]).await.is_empty());
        assert_eq!(
            observed(&passes.site),
            progress,
            "stopped passes do not change counters"
        );
        assert_eq!(
            passes.checkpoints[0]
                .iter()
                .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                .collect::<Vec<_>>(),
            [CLEAN, CLEAN, OWED]
        );
    }

    #[tokio::test]
    async fn a_panicking_request_with_an_empty_label_is_attempted_not_skipped() {
        let mut minute = named_leg("zerodha", "1min", "minute");
        minute.label.clear();
        let option = named_leg("zerodha", "options", "option");
        let mut passes = Passes::new(
            "checkpoint-empty-label-panic",
            vec![minute.clone(), option.clone()],
        );
        passes
            .run(|_site, _leg| async {
                panic!("synthetic first-leg panic");
                #[allow(unreachable_code)]
                receipt(200, "STORED")
            })
            .await;
        let progress = observed(&passes.site);
        let feed = &progress.feeds[0];
        assert_eq!((feed.legs_done, feed.skipped, feed.retries), (0, 1, 0));
        assert!(feed.finished && feed.doing.is_empty());
        assert_eq!(passes.outcomes, [PassOutcome::Retry]);
        assert!(feed.last_error.as_ref().unwrap().contains("abnormally"));
        assert_eq!(passes.respond(&[]).await, vec![minute, option]);
        assert_eq!(observed(&passes.site).retries, 1);
        assert_eq!(passes.outcomes, [PassOutcome::Clean]);
    }

    #[tokio::test]
    async fn stop_before_a_queued_retry_starts_does_not_count_it_as_attempted() {
        let mut passes = Passes::new(
            "checkpoint-stop-queued",
            vec![
                named_leg("zerodha", "1day", "z"),
                named_leg("groww", "1day", "g"),
            ],
        );
        assert_eq!(
            passes
                .respond(&[("z", 503, "FAILED"), ("g", 503, "FAILED")])
                .await
                .len(),
            2
        );
        // The default current-thread runtime cannot interleave these ready
        // requests: whichever task starts first sets stop before it returns.
        passes
            .run(|site, _leg| {
                edit_slot(&site, |p| p.stopping = true);
                std::future::ready(receipt(200, "STORED"))
            })
            .await;
        let progress = observed(&passes.site);
        assert_eq!(progress.retries, 1);
        assert_eq!(progress.feeds.iter().map(|f| f.retries).sum::<u32>(), 1);
        assert_eq!(progress.feeds.iter().map(|f| f.legs_done).sum::<u32>(), 1);
        assert_eq!(progress.feeds.iter().map(|f| f.skipped).sum::<u32>(), 1);
        assert!(
            progress
                .feeds
                .iter()
                .all(|f| f.finished && f.doing.is_empty())
        );
        assert!(passes.respond(&[]).await.is_empty());
        assert_eq!(observed(&passes.site), progress);
    }

    /// A receipt that states its retry class, as the server writes it.
    fn classed(status: u16, verdict: &str, class: &str) -> (axum::http::StatusCode, String) {
        (
            axum::http::StatusCode::from_u16(status).expect("HTTP status"),
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict,
                reason: "synthetic coordinator receipt",
                good: false,
                facts: &[(
                    RETRY_CLASS_FACT,
                    format!("{class} — 0 failure(s) another attempt could change"),
                )],
                footnote: "No vendor was contacted.",
            }),
        )
    }

    /// Writes a census of `rows` rows for `vendor` into the site's store.
    fn write_census(site: &Site, vendor: brutex_core::vendor::Vendor, rows: u64) {
        use brutex_core::instrument::{Exchange, Segment};
        use brutex_core::symbol::Symbol;
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        use store::path::{Timeframe, YearMonth};

        let mut census = Manifest::open(vendor, &[], &[]).expect("empty fixture");
        census
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: Symbol::new("NIFTY").expect("symbol"),
                    timeframe: Timeframe::MINUTE_1,
                    month: YearMonth::new(2025, 7).expect("month"),
                },
                rows,
                first_ts_micros: 1,
                last_ts_micros: 1,
            })
            .expect("record counter");
        std::fs::write(manifest_path(&site.store_root, vendor), census.image())
            .expect("write fixture census");
    }

    /// **ANOTHER WRITER'S ROWS ARE NOT THIS RUN'S PROGRESS** (DB-B33).
    ///
    /// A GDFL census grows on every request while a Zerodha run's receipts
    /// are all clean. The growth test summed every vendor, so each pass read
    /// as progress and the run never stopped idle (it ran to the pass
    /// ceiling, and reported the other feed's rows as bars it added). Counted
    /// on the run's own vendor it stops after the three idle passes with
    /// nothing added.
    #[tokio::test]
    async fn another_vendors_growth_neither_keeps_the_run_alive_nor_counts_as_its_bars() {
        let grown = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let grow = std::sync::Arc::clone(&grown);
        let progress = simulated(
            "foreign-growth",
            vec![leg("zerodha", "1day")],
            move |site, _leg| {
                let rows = grow.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                write_census(&site, brutex_core::vendor::Vendor::Gdfl, rows);
                std::future::ready(receipt(200, "STORED"))
            },
        )
        .await;
        assert_eq!(
            grown.load(std::sync::atomic::Ordering::Relaxed),
            u64::from(CLEAN_EMPTY_PASSES),
            "one request a pass, and the idle stop after three"
        );
        assert_eq!(progress.passes, CLEAN_EMPTY_PASSES);
        assert_eq!(
            (progress.rows_at_start, progress.rows_now),
            (Some(0), Some(0)),
            "the other vendor's rows are not on this run's counter"
        );
        assert_eq!(progress.stop, Some(Stop::Idle));
        let summary = progress.finished.expect("summary");
        assert!(
            summary.contains("Idle retries stopped") && summary.contains("; 0 bar(s) added"),
            "{summary}"
        );
    }

    /// The run's own vendor still counts: the same loop, growing Zerodha.
    #[tokio::test]
    async fn the_runs_own_vendor_growth_is_still_counted() {
        let site = site("own-growth");
        write_census(&site, brutex_core::vendor::Vendor::Gdfl, 9_000);
        write_census(&site, brutex_core::vendor::Vendor::Zerodha, 5);
        let own = own_vendors(&by_feed(vec![leg("zerodha", "1day")]));
        assert_eq!(own, [brutex_core::vendor::Vendor::Zerodha]);
        assert_eq!(rows_of(&site, &own), Some(5));
        assert_eq!(
            rows_now(&site),
            Some(9_005),
            "the all-vendor total, for contrast"
        );
        assert_eq!(
            rows_of(&site, &own_vendors(&by_feed(vec![leg("gdfl", "1sec")]))),
            Some(9_000)
        );
        assert_eq!(
            rows_of(&site, &[]),
            Some(0),
            "no vendor of its own proves no growth"
        );
    }

    /// **A LEG WHOSE FAILURES ARE ALL STANDING IS SETTLED, NOT ASKED AGAIN**
    /// (DB-B26). The minute leg answers HTTP 200 FAILED with every failure
    /// classed standing: the vendor answered and what it holds has gaps. It
    /// was Retry, pulled again whole every 20 s up to 400 passes; now it is
    /// asked once, the daily leg alone is asked on the later idle passes, and
    /// the run ends naming the standing failures.
    #[tokio::test]
    async fn a_leg_with_only_standing_failures_is_asked_once_and_the_run_stops_idle() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "standing-leg",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |_site, leg| {
                capture.lock().expect("calls").push(leg.dir.clone());
                std::future::ready(if leg.dir == "1min" {
                    classed(200, "FAILED", RetryClass::Standing.word())
                } else {
                    receipt(200, "STORED")
                })
            },
        )
        .await;
        assert_eq!(
            *calls.lock().expect("calls"),
            ["1day", "1min", "1day", "1day"],
            "the standing leg is asked exactly once"
        );
        assert_eq!((progress.passes, progress.retries), (CLEAN_EMPTY_PASSES, 0));
        assert_eq!(progress.feeds[0].standing, 1);
        assert_eq!(progress.stop, Some(Stop::Standing));
        let summary = progress.finished.expect("summary");
        assert!(
            summary.contains("FINISHED WITH STANDING FAILURES")
                && summary.contains("zerodha (1 leg(s))")
                && summary.contains("is not asked again"),
            "{summary}"
        );
        let doc = progress_doc(&progress_with(progress.passes, Stop::Standing));
        assert!(
            doc.contains("\"stopReason\":\"standing_failures\""),
            "{doc}"
        );
        assert!(
            doc.contains(&format!("\"passCeiling\":{MAX_PASSES}")),
            "{doc}"
        );
    }

    fn progress_with(passes: u32, stop: Stop) -> Progress {
        Progress {
            passes,
            stop: Some(stop),
            finished: Some("done".to_owned()),
            ..Progress::default()
        }
    }

    fn progress_doc(progress: &Progress) -> String {
        progress.json()
    }

    /// The pull order's whole-run refusal (409 with a standing class) is
    /// settled too: it was read as a busy seat and asked again 400 times
    /// (DB-B27). A 409 that states no class is still a busy seat.
    #[test]
    fn the_receipts_class_decides_between_retry_settled_and_halted() {
        let (ok, standing) = classed(200, "FAILED", RetryClass::Standing.word());
        assert_eq!(leg_outcome(ok, &standing), LegOutcome::Standing);
        let (conflict, order) = classed(409, "REFUSED", RetryClass::Standing.word());
        assert_eq!(leg_outcome(conflict, &order), LegOutcome::Standing);
        let (ok, transient) = classed(200, "FAILED", RetryClass::Transient.word());
        assert_eq!(leg_outcome(ok, &transient), LegOutcome::Retry);
        let (ok, store) = classed(200, "FAILED", RetryClass::Store.word());
        assert_eq!(leg_outcome(ok, &store), LegOutcome::Store);
        let (conflict, busy) = receipt(409, "REFUSED");
        assert_eq!(leg_outcome(conflict, &busy), LegOutcome::Retry);
        let (ok, unclassed) = receipt(200, "PARTIAL");
        assert_eq!(leg_outcome(ok, &unclassed), LegOutcome::Retry);
        // A class never outranks a clean badge or a fixed refusal.
        let (ok, stored) = classed(200, "STORED", RetryClass::Standing.word());
        assert_eq!(
            RetryClass::of_receipt(&stored),
            Some(RetryClass::Standing),
            "the fact is read off the page"
        );
        let (missing, gone) = classed(404, "FAILED", RetryClass::Standing.word());
        assert_eq!(leg_outcome(missing, &gone), LegOutcome::Permanent);
        let _ = ok;
        assert_eq!(RetryClass::of_receipt("<html></html>"), None);
        assert_eq!(
            RetryClass::of_receipt("<tr><th>Retry class</th><td>garbage words</td></tr>"),
            None,
            "an unknown word is no class"
        );
    }

    /// **A STORE THAT WILL NOT TAKE A WRITE HALTS THE FEED** (DB-B35): the
    /// leg after it is never asked, nothing is retried, and the stop is
    /// typed.
    #[tokio::test]
    async fn a_store_unavailable_receipt_halts_the_feed_and_types_the_stop() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "store-unavailable",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |_site, leg| {
                capture.lock().expect("calls").push(leg.dir.clone());
                std::future::ready(classed(200, "FAILED", RetryClass::Store.word()))
            },
        )
        .await;
        assert_eq!(*calls.lock().expect("calls"), ["1day"]);
        assert_eq!((progress.passes, progress.retries), (1, 0));
        assert!(progress.feeds[0].store_unavailable);
        assert_eq!(progress.stop, Some(Stop::StoreUnavailable));
        let said = progress.feeds[0].last_error.clone().expect("the reason");
        assert!(said.contains("STORE UNAVAILABLE"), "{said}");
        let summary = progress.finished.expect("summary");
        assert!(
            summary.starts_with("INCOMPLETE after 1 pass(es)"),
            "{summary}"
        );
    }

    /// Every stop has its own code, and the order of precedence holds.
    #[test]
    fn the_stop_reason_is_typed_and_ordered() {
        let codes: Vec<&str> = [
            Stop::Idle,
            Stop::Standing,
            Stop::Ceiling,
            Stop::Operator,
            Stop::Halted,
            Stop::StoreUnavailable,
        ]
        .into_iter()
        .map(Stop::code)
        .collect();
        assert_eq!(
            codes,
            [
                "idle",
                "standing_failures",
                "pass_ceiling",
                "stopped",
                "halted",
                "store_unavailable"
            ]
        );
        let feed = |standing, store_unavailable, credential_dead| FeedReport {
            credential_dead,
            standing,
            store_unavailable,
            ..FeedReport::default()
        };
        let of_idle = |passes, stopping, feeds: Vec<FeedReport>, outcomes: &[PassOutcome], idle| {
            stop_of(
                &Progress {
                    passes,
                    feeds,
                    stopping,
                    ..Progress::default()
                },
                outcomes,
                idle,
            )
        };
        let of = |passes, stopping, feeds: Vec<FeedReport>, outcomes: &[PassOutcome]| {
            of_idle(passes, stopping, feeds, outcomes, passes < MAX_PASSES)
        };
        let clean = [PassOutcome::Clean];
        assert_eq!(
            of(3, false, vec![feed(0, false, false)], &clean),
            Stop::Idle
        );
        assert_eq!(
            of(3, false, vec![feed(2, false, false)], &clean),
            Stop::Standing
        );
        assert_eq!(
            of(MAX_PASSES, false, vec![feed(2, false, false)], &clean),
            Stop::Ceiling
        );
        // Idle on exactly the last allowed pass is idle, not the ceiling.
        assert_eq!(
            of_idle(MAX_PASSES, false, vec![feed(0, false, false)], &clean, true),
            Stop::Idle
        );
        assert_eq!(
            of_idle(MAX_PASSES, false, vec![feed(2, false, false)], &clean, true),
            Stop::Standing
        );
        assert_eq!(
            of(MAX_PASSES, false, vec![feed(0, false, true)], &clean),
            Stop::Halted
        );
        assert_eq!(
            of(
                3,
                false,
                vec![feed(0, false, false)],
                &[PassOutcome::Halted]
            ),
            Stop::Halted
        );
        assert_eq!(
            of(3, false, vec![feed(0, true, true)], &[PassOutcome::Halted]),
            Stop::StoreUnavailable
        );
        assert_eq!(
            of(3, true, vec![feed(0, true, true)], &[PassOutcome::Halted]),
            Stop::Operator
        );
        assert!(Progress::default().json().contains("\"stopReason\":null"));
    }

    fn opened_at(secs: u64, source: &str, from: u32, to: u32) -> crate::audit::Record {
        let day = |at| pull::session::Day::from_days(at).expect("a day");
        crate::audit::Record::opened(
            crate::audit::Scope::Spot,
            std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs),
            source,
            pull::session::Window::new(day(from), day(to)).expect("a window"),
            7,
        )
    }

    fn closed_at(secs: u64, source: &str, from: u32, to: u32) -> crate::audit::Record {
        let day = |at| pull::session::Day::from_days(at).expect("a day");
        crate::audit::Record::refused(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::Failed,
            std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs),
            source,
            "closed",
        )
        .with_window(pull::session::Window::new(day(from), day(to)).expect("a window"))
    }

    fn failure_at(secs: u64) -> crate::audit::Record {
        let day = pull::session::Day::from_days(20_000).expect("a day");
        crate::audit::Record::member_failure(
            crate::audit::Scope::Spot,
            std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs),
            "NIFTY",
            pull::session::Window::new(day, day).expect("a window"),
            "a gap",
        )
    }

    fn numbered(
        records: Vec<crate::audit::Record>,
        first: u64,
    ) -> Vec<(u64, crate::audit::Record)> {
        records
            .into_iter()
            .enumerate()
            .map(|(at, record)| (first + at as u64, record))
            .collect()
    }

    /// **AN OPENING RECORD WITH NO CLOSING RECORD IS A RUN CUT SHORT**
    /// (DB-B34), in every shape the journal can hold it: empty, closed,
    /// open, and two runs.
    #[test]
    fn a_run_the_journal_never_closed_is_found_and_a_closed_one_is_not() {
        assert_eq!(interrupted_in(&[]), None, "an empty journal");
        let closed = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                closed_at(120, "kite", 20_000, 20_010),
            ],
            0,
        );
        assert_eq!(interrupted_in(&closed), None, "a closed run");

        let open = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                failure_at(130),
            ],
            40,
        );
        let found = interrupted_in(&open).expect("an open run");
        assert_eq!(
            (
                found.ordinal,
                found.opened_at_secs,
                found.last_at_secs,
                found.failures,
                found.failures_whole
            ),
            (40, 100, 130, 2, true)
        );
        assert_eq!(found.window, Some((20_000, 20_010)));
        assert_eq!(found.source, "kite");
        assert!(
            found.note.starts_with("opened for 7 instruments"),
            "{}",
            found.note
        );
        let words = found.words();
        assert!(
            words.starts_with(
                "INTERRUPTED: a pull run for 2024-10-04..2024-10-14 was opened at journal record 40"
            ) && words.contains("with 2 member failure(s) on record"),
            "{words}"
        );
        // It does not claim to know WHICH process stopped: a `cli` pull still
        // in flight has no closing record either.
        assert!(
            !words.contains("The server stopped while it ran")
                && words.contains("unless another process"),
            "{words}"
        );
    }

    /// **MEMBER FAILURES ARE NO EVIDENCE OF AN OPEN RUN** (DB-B34, repair 1).
    /// A run's failures can lie BEHIND its closing record: every run written
    /// before D-3341 is laid out so, and `RunLedger::rest` still writes that
    /// layout when the opening record was refused. The walk read failures
    /// with no run record before them as "a run opened further back and
    /// never closed", and said INTERRUPTED of a journal no run was open in.
    #[test]
    fn failures_behind_a_closing_record_or_ahead_of_any_run_record_open_no_run() {
        // The operator's own layout: [closed][failures ...].
        let mut behind = vec![closed_at(120, "kite", 20_000, 20_010)];
        behind.extend((0..850).map(|at| failure_at(121 + at)));
        assert_eq!(interrupted_in(&numbered(behind, 1_804_099)), None);
        // Only failures: whose they are is not in the records read.
        let headless = numbered(vec![failure_at(500), failure_at(510)], 5_000);
        assert_eq!(
            interrupted_in(&headless),
            None,
            "failures alone never make a run"
        );
        // ... and an OPENED record with as many failures behind it is found,
        // with its own window and source.
        let mut open = vec![opened_at(100, "kite", 20_000, 20_010)];
        open.extend((0..850).map(|at| failure_at(121 + at)));
        let found = interrupted_in(&numbered(open, 7)).expect("the run is open");
        assert_eq!(
            (
                found.ordinal,
                found.failures,
                found.window,
                found.source.as_str()
            ),
            (7, 850, Some((20_000, 20_010)), "kite")
        );
    }

    /// The same walk over the shapes two writers leave: a head restated after
    /// another feed's records, a failure offered again behind the run's own
    /// close, a refusal of another request for the same window, and two runs
    /// of which only the second is open (DB-B34).
    #[test]
    fn interleaved_restated_and_refused_records_are_told_apart() {
        // A second feed appended in between, so the first restated its head.
        let mut again = opened_at(100, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 0".to_owned();
        let interleaved = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                opened_at(111, "gdfl", 20_000, 20_010),
                closed_at(112, "gdfl", 20_000, 20_010),
                again.clone(),
                failure_at(140),
            ],
            0,
        );
        let found = interrupted_in(&interleaved).expect("the first feed's run is open");
        assert_eq!(
            (
                found.ordinal,
                found.failures,
                found.last_at_secs,
                found.source.as_str()
            ),
            (0, 2, 140, "kite"),
            "the other feed's close ended the other feed's run"
        );

        // A failure offered again BEHIND the run's own closing record.
        let after_close = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                closed_at(120, "kite", 20_000, 20_010),
                again.clone(),
                failure_at(121),
            ],
            0,
        );
        assert_eq!(interrupted_in(&after_close), None);
        // The same, with the opening record before the records read.
        let unseen = numbered(
            vec![
                closed_at(120, "kite", 20_000, 20_010),
                again.clone(),
                failure_at(121),
            ],
            900,
        );
        assert_eq!(interrupted_in(&unseen), None);
        // A restated head with NO close before it stands for its opening
        // record, which lies before the records read: the run is open, with
        // the opening record's own ordinal, window and source.
        let restated_only = numbered(vec![again.clone(), failure_at(141)], 900);
        let found = interrupted_in(&restated_only).expect("the restated run is open");
        assert_eq!(
            (
                found.ordinal,
                found.failures,
                found.failures_whole,
                found.window
            ),
            (0, 1, false, Some((20_000, 20_010)))
        );

        // ANOTHER REQUEST'S REFUSAL FOR THE SAME WINDOW closes nothing: a
        // request refused before anything ran never opened a run.
        let mut refusal = closed_at(115, "fno", 20_000, 20_010);
        refusal.outcome = crate::audit::Outcome::Refused;
        let refused_beside = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                refusal,
                failure_at(116),
            ],
            0,
        );
        let found = interrupted_in(&refused_beside).expect("the run is still open");
        assert_eq!(
            (found.ordinal, found.failures),
            (0, 1),
            "the failure behind the refusal is the refusal's, not the run's"
        );
        // A run that reached nobody is closed under its TARGET's name
        // (NotStarted), not its endpoint's: the window alone pairs them.
        let mut reached_nobody = closed_at(118, "fno", 20_000, 20_010);
        reached_nobody.outcome = crate::audit::Outcome::NotStarted;
        assert_eq!(
            interrupted_in(&numbered(
                vec![opened_at(100, "kite", 20_000, 20_010), reached_nobody],
                0
            )),
            None
        );

        // Two runs: the first closed, the second cut short.
        let two = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                closed_at(120, "kite", 20_000, 20_010),
                opened_at(200, "kite", 20_000, 20_010),
                failure_at(210),
            ],
            0,
        );
        assert_eq!(
            interrupted_in(&two).map(|run| (run.ordinal, run.failures)),
            Some((2, 1))
        );
    }

    /// **THE SCAN ON DISK, PAST ITS 800 RECORDS** (DB-B34, repair 1): the
    /// run record that settles the question is read through the journal's
    /// own index, however many failures follow it.
    ///
    /// * `[closed][850 failures]`, the operator's journal in small: CLEAN.
    ///   It was INTERRUPTED "opened further back than the last 800 records".
    /// * `[opened][850 failures]`: INTERRUPTED, with the opening record's
    ///   true ordinal, window and source. They were all `null`.
    /// * `[opened][closed][850 failures]`: CLEAN, both read by ordinal.
    #[test]
    fn the_restart_scan_reads_the_newest_run_record_through_the_index() {
        let over = INTERRUPTED_SCAN + 50;
        let with_failures = |head: crate::audit::Record, failures: u64| {
            let mut records = vec![head];
            records.extend((0..failures).map(|at| failure_at(200 + at)));
            records
        };

        // The closed run with its failures behind it.
        let closed = journal_of(
            "pullrun-scan-closed-behind",
            &with_failures(closed_at(120, "kite", 20_000, 20_010), over),
        );
        assert_eq!(restart_scan(&closed), Restart::Clean);
        // ... and it stays clean one short of the scan, where the whole
        // journal is read, and exactly at it, where the run record is the
        // one record before the scan.
        for failures in [INTERRUPTED_SCAN - 1, INTERRUPTED_SCAN] {
            let journal = journal_of(
                "pullrun-scan-closed-edge",
                &with_failures(closed_at(120, "kite", 20_000, 20_010), failures),
            );
            assert_eq!(
                restart_scan(&journal),
                Restart::Clean,
                "{failures} failures"
            );
        }

        // The open run with as many failures behind it.
        let open = journal_of(
            "pullrun-scan-open-behind",
            &with_failures(opened_at(100, "kite", 20_000, 20_010), over),
        );
        let found = restart_scan(&open);
        let run = found.interrupted().expect("the run is open");
        assert_eq!(
            (
                run.ordinal,
                run.opened_at_secs,
                run.window,
                run.source.as_str(),
                run.failures,
                run.failures_whole
            ),
            (
                0,
                100,
                Some((20_000, 20_010)),
                "kite",
                INTERRUPTED_SCAN,
                false
            ),
            "the opening record is read by its ordinal; the failures counted are the scan's"
        );
        assert!(
            run.words()
                .contains(&format!("at least {INTERRUPTED_SCAN} member failure(s)")),
            "{}",
            run.words()
        );
        let doc: serde_json::Value =
            serde_json::from_str(&status_json(&Progress::default(), None, Some(&found)))
                .expect("valid JSON");
        assert_eq!(doc["restart"]["state"], "interrupted");
        assert_eq!(doc["interrupted"]["ordinal"], 0);
        assert_eq!(doc["interrupted"]["from"], "2024-10-04");
        assert_eq!(doc["interrupted"]["failuresWhole"], false);

        // A closing record AFTER the opening one, both before the scan.
        let mut both = with_failures(opened_at(100, "kite", 20_000, 20_010), 2);
        both.push(closed_at(130, "kite", 20_000, 20_010));
        both.extend((0..over).map(|at| failure_at(300 + at)));
        assert_eq!(
            restart_scan(&journal_of("pullrun-scan-open-then-closed", &both)),
            Restart::Clean
        );
    }

    /// **AN INDEX THAT IS NOT THERE IS BUILT WHERE IT IS NEEDED** (DB-B34,
    /// repair 1), so the answer is the same exact one on the first start
    /// over a copied journal; and where it cannot be built, NOT KNOWN is
    /// said, with both reasons, never a guess either way.
    #[test]
    fn an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be() {
        use std::os::unix::fs::PermissionsExt as _;
        let over = INTERRUPTED_SCAN + 50;
        let with_failures = |head: crate::audit::Record, failures: u64| {
            let mut records = vec![head];
            records.extend((0..failures).map(|at| failure_at(200 + at)));
            records
        };
        let closed = journal_of(
            "pullrun-scan-absent-closed",
            &with_failures(closed_at(120, "kite", 20_000, 20_010), over),
        );
        let open = journal_of(
            "pullrun-scan-absent-open",
            &with_failures(opened_at(100, "kite", 20_000, 20_010), over),
        );
        for (journal, clean) in [(&closed, true), (&open, false)] {
            let index = journal.index_dir();
            let aside = index.with_extension("aside");
            std::fs::rename(&index, &aside).expect("the index is moved aside");
            let found = restart_scan(journal);
            assert_eq!(found == Restart::Clean, clean, "{found:?}");
            assert_eq!(
                found.interrupted().map(|run| (run.ordinal, run.window)),
                (!clean).then_some((0, Some((20_000, 20_010))))
            );
            assert!(index.join("cover").is_file(), "the index was built");
        }

        // WHERE IT CANNOT BE BUILT, NOT KNOWN IS SAID, for both journals and
        // for both reasons: another writer holds the journal's lock, and a
        // store that will not take the write.
        for (journal, name) in [
            (&closed, "pullrun-scan-absent-closed"),
            (&open, "pullrun-scan-absent-open"),
        ] {
            let index = journal.index_dir();
            let path = crate::audit::journal_path(&crate::scratch::path(name));
            std::fs::rename(&index, index.with_extension("aside2")).expect("moved aside");
            let squatter = store::flock::Flock::try_lock(
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .expect("the journal handle"),
                path.clone(),
            )
            .expect("nothing else holds it");
            let held = restart_scan_waiting(journal, 2);
            drop(squatter);
            let Restart::NotKnown(why) = &held else {
                panic!("with the lock held nothing is known: {held:?}");
            };
            assert!(
                why.contains("no index at")
                    && why.contains("another writer held the journal's lock through 2 wait(s)"),
                "{why}"
            );
            let doc: serde_json::Value =
                serde_json::from_str(&status_json(&Progress::default(), None, Some(&held)))
                    .expect("valid JSON");
            assert_eq!(doc["restart"]["state"], "not_known");
            assert_eq!(doc["interrupted"], serde_json::Value::Null);
            let words = doc["restart"]["words"].as_str().expect("words");
            assert!(
                words.starts_with("NOT KNOWN whether a pull run was left open")
                    && !words.contains("INTERRUPTED"),
                "{words}"
            );

            let audit = path.parent().expect("audit/").to_path_buf();
            std::fs::set_permissions(&audit, std::fs::Permissions::from_mode(0o555))
                .expect("chmod");
            let refused = restart_scan(journal);
            std::fs::set_permissions(&audit, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
            let Restart::NotKnown(why) = &refused else {
                panic!("an index that cannot be written leaves nothing known: {refused:?}");
            };
            assert!(why.contains("could not be built here"), "{why}");
        }

        // With no index, a run record INSIDE the scan settles it either way:
        // a restated head stands for its opening record, and a closing
        // record after it closes the run.
        let mut again = opened_at(100, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 0".to_owned();
        open.append(&again).expect("the restated head lands");
        assert_eq!(
            restart_scan(&open).interrupted().map(|run| run.ordinal),
            Some(0)
        );
        open.append(&closed_at(2_000, "kite", 20_000, 20_010))
            .expect("the closing record lands");
        assert_eq!(restart_scan(&open), Restart::Clean);
    }

    /// **A FALSE "INTERRUPTED" DOES NOT OUTLIVE THE RUNS AFTER IT** and a
    /// true one is not hidden by them: later runs that open and close leave
    /// an older open run open, and a journal whose every run closed clean.
    #[test]
    fn later_closed_runs_neither_invent_nor_hide_an_open_run() {
        let mut records = vec![closed_at(120, "kite", 20_000, 20_010)];
        records.extend((0..INTERRUPTED_SCAN + 50).map(|at| failure_at(200 + at)));
        let journal = journal_of("pullrun-scan-later-runs", &records);
        for later in 0..10_u64 {
            let at = 5_000 + later * 10;
            let _thread = journal
                .open_run(&opened_at(at, "kite", 20_020, 20_030))
                .expect("opened");
            journal
                .append(&closed_at(at + 5, "kite", 20_020, 20_030))
                .expect("closed");
            assert_eq!(restart_scan(&journal), Restart::Clean, "after {later} runs");
        }
        // One more opens and is not closed: found, though ten closed after
        // the journal's first.
        let _thread = journal
            .open_run(&opened_at(9_000, "kite", 20_040, 20_050))
            .expect("opened");
        let found = restart_scan(&journal);
        assert_eq!(
            found.interrupted().map(|run| run.window),
            Some(Some((20_040, 20_050)))
        );
    }

    /// Overwrites record `ordinal` of `journal` with bytes no build decodes,
    /// leaving the journal's length and its index as they are: a record
    /// damaged on disk after it was written and indexed.
    fn damage(journal: &crate::audit::Journal, ordinal: u64) {
        use std::os::unix::fs::FileExt as _;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(journal.index_dir().with_extension(""))
            .expect("the journal opens");
        file.write_all_at(
            &[0xFF_u8; crate::audit::RECORD_LEN],
            ordinal * crate::audit::RECORD_LEN as u64,
        )
        .expect("the record is overwritten");
    }

    /// The reason of a NOT KNOWN answer, or a failure naming what was
    /// answered instead.
    fn not_known_of(found: &Restart) -> &str {
        match found {
            Restart::NotKnown(why) => why,
            other => panic!("NOT KNOWN was expected, and the scan said {other:?}"),
        }
    }

    /// **A RECORD THE SCAN CANNOT DECODE IS NEVER DROPPED IN SILENCE**
    /// (DB-B34, repair 2). The scan filtered such records out and answered
    /// as if it had read everything, in three shapes a second agent drove:
    ///
    /// * `[opened][5 failures]` with the OPENING record damaged read CLEAN
    ///   ("No pull run was left open") while a run was open;
    /// * `[opened][closed][3 failures]` with the CLOSING record damaged read
    ///   INTERRUPTED ("has no closing record") of a run that was closed;
    /// * `[opened][850 failures]` with the opening record damaged and the
    ///   index still naming it as a run record read CLEAN.
    ///
    /// Each is NOT KNOWN now, naming the record and the decoder's reason and
    /// saying what the readable half says; and the answer is logged at Warn.
    #[test]
    fn a_record_that_cannot_be_decoded_is_named_and_the_answer_is_not_known() {
        let with_failures = |mut records: Vec<crate::audit::Record>, failures: u64| {
            records.extend((0..failures).map(|at| failure_at(200 + at)));
            records
        };
        let opened = || opened_at(100, "kite", 20_000, 20_010);
        let closed = || closed_at(120, "kite", 20_000, 20_010);

        // (a) The opening record, inside the scan.
        let journal = journal_of("pullrun-damaged-open", &with_failures(vec![opened()], 5));
        assert_eq!(
            restart_scan(&journal)
                .interrupted()
                .map(|run| (run.ordinal, run.failures)),
            Some((0, 5)),
            "whole, the journal says a run is open"
        );
        damage(&journal, 0);
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(why.contains("1 of the journal records"), "{why}");
        assert!(why.contains("record 0 (not an audit record)"), "{why}");
        assert!(why.contains("leave no run open"), "{why}");
        assert_eq!(found.word(), "not_known");
        assert_eq!(found.interrupted(), None);

        // (b) The closing record: the run WAS closed, and "has no closing
        // record" is never said of it.
        let journal = journal_of(
            "pullrun-damaged-close",
            &with_failures(vec![opened(), closed()], 3),
        );
        assert_eq!(restart_scan(&journal), Restart::Clean);
        damage(&journal, 1);
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(why.contains("record 1 (not an audit record)"), "{why}");
        assert!(
            why.contains("leave the run opened at record 0 open")
                && why.contains("may be its closing record"),
            "{why}"
        );
        assert!(!found.words().contains("has no closing record"));

        // (c) The opening record behind the scan, reached through the index,
        // which still names it as a run record.
        let over = INTERRUPTED_SCAN + 50;
        let journal = journal_of(
            "pullrun-damaged-indexed",
            &with_failures(vec![opened()], over),
        );
        assert_eq!(
            restart_scan(&journal).interrupted().map(|run| run.ordinal),
            Some(0)
        );
        damage(&journal, 0);
        assert_eq!(
            journal
                .read_index(over + 1)
                .expect("the index reads")
                .runs
                .iter()
                .map(|run| run.0)
                .collect::<Vec<_>>(),
            vec![0],
            "the index is untouched and still names record 0 as a run record"
        );
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(why.contains("record 0 (not an audit record)"), "{why}");
        assert!(why.contains("leave no run open"), "{why}");

        // (d) The same journal with its index REBUILT after the damage: the
        // record is filed under `faults`, not `runs`, and nobody hands it to
        // the scan. It is still found and still named.
        std::fs::rename(
            journal.index_dir(),
            journal.index_dir().with_extension("old"),
        )
        .expect("the index is moved aside");
        let found = restart_scan(&journal);
        let rebuilt = journal.read_index(over + 1).expect("the index was rebuilt");
        assert_eq!(
            (rebuilt.runs.len(), rebuilt.fault_ordinals.clone()),
            (0, vec![0])
        );
        let why = not_known_of(&found);
        assert!(why.contains("record 0 (not an audit record)"), "{why}");

        // (e) A damaged MEMBER FAILURE in the scan is no different: nothing
        // says it was one.
        let journal = journal_of(
            "pullrun-damaged-member",
            &with_failures(vec![opened(), closed()], 6),
        );
        damage(&journal, 5);
        let why_text = restart_scan(&journal);
        assert!(not_known_of(&why_text).contains("record 5 (not an audit record)"));

        // (f) MANY: the sentence names three and counts the rest.
        let journal = journal_of("pullrun-damaged-many", &with_failures(vec![closed()], 9));
        for ordinal in 2..=8 {
            damage(&journal, ordinal);
        }
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(why.starts_with("7 of the journal records"), "{why}");
        assert!(
            why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
            "{why}"
        );
        assert!(why.contains("and 4 more, the newest record 8"), "{why}");
    }

    /// **DAMAGE THAT CANNOT DECIDE THE ANSWER DOES NOT HIDE AN OPEN RUN**
    /// (DB-B34, repair 2): a record that will not decode BEFORE the opening
    /// record of the run left open can be neither that run's close nor a
    /// newer run, so the run is still reported; the same damage AFTER the
    /// opening record is NOT KNOWN. A restated opening record whose own
    /// first record is the damaged one still names the run.
    #[test]
    fn damage_before_the_open_runs_first_record_leaves_the_run_reported() {
        let records = vec![
            opened_at(100, "kite", 20_000, 20_010),
            closed_at(120, "kite", 20_000, 20_010),
            failure_at(130),
            opened_at(200, "kite", 20_020, 20_030),
            failure_at(210),
            failure_at(211),
        ];
        for before in [0, 1, 2] {
            let journal = journal_of(&format!("pullrun-damage-before-{before}"), &records);
            damage(&journal, before);
            assert_eq!(
                restart_scan(&journal).interrupted().map(|run| (
                    run.ordinal,
                    run.window,
                    run.failures
                )),
                Some((3, Some((20_020, 20_030)), 2)),
                "record {before} lies before the open run's first record"
            );
        }
        let journal = journal_of("pullrun-damage-after", &records);
        damage(&journal, 4);
        let found = restart_scan(&journal);
        assert!(
            not_known_of(&found).contains("leave the run opened at record 3 open"),
            "{found:?}"
        );

        // The opening record itself is damaged and a restated one names it:
        // the run is still reported, at its true ordinal.
        let mut again = opened_at(100, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 0".to_owned();
        let journal = journal_of(
            "pullrun-damage-restated",
            &[
                opened_at(100, "kite", 20_000, 20_010),
                again.clone(),
                failure_at(140),
            ],
        );
        damage(&journal, 0);
        assert_eq!(
            restart_scan(&journal)
                .interrupted()
                .map(|run| (run.ordinal, run.failures)),
            Some((0, 1))
        );
        // ... and damage between the two may be the close that makes the
        // restatement no new run: NOT KNOWN.
        let journal = journal_of(
            "pullrun-damage-restated-between",
            &[
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                again,
                failure_at(140),
            ],
        );
        damage(&journal, 1);
        assert_eq!(restart_scan(&journal).word(), "not_known");
    }

    /// **AN INDEX THAT LOST AN ENTRY IS NOT READ AS A WHOLE ONE** (DB-B34,
    /// repair 2): a `runs` entry that cannot be read may be the newest run
    /// record, so the run records before the scan are not known through it,
    /// and with no run record in the scan the answer is NOT KNOWN. It read
    /// "clean" of a journal a run was open in.
    #[test]
    fn an_index_entry_that_cannot_be_read_is_not_a_run_record_that_is_not_there() {
        let mut records = vec![opened_at(100, "kite", 20_000, 20_010)];
        records.extend((0..INTERRUPTED_SCAN + 50).map(|at| failure_at(200 + at)));
        let journal = journal_of("pullrun-index-entry-damaged", &records);
        assert_eq!(
            restart_scan(&journal).interrupted().map(|run| run.ordinal),
            Some(0)
        );
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        assert_eq!(bytes.len(), 32, "one run record, one entry");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entry is overwritten");
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(
            why.contains("1 of the index's entries cannot be read"),
            "{why}"
        );
    }

    /// **A RESTATED HEAD REJOINS ITS OWN OPEN RUN** whatever closed before
    /// it: an earlier closing record of the same window and source, whose
    /// opening record was not read, made the restatement "no new run" and
    /// the failures behind it were counted to nobody.
    #[test]
    fn a_restated_head_of_an_open_run_rejoins_it_after_an_older_runs_close() {
        let mut again = opened_at(300, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 1".to_owned();
        let read = numbered(
            vec![
                closed_at(90, "kite", 20_000, 20_010),
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                again,
                failure_at(310),
                failure_at(311),
            ],
            0,
        );
        assert_eq!(
            interrupted_in(&read).map(|run| (run.ordinal, run.failures)),
            Some((1, 3))
        );
    }

    /// A journal of exactly `records`, written in one write and indexed in
    /// one pass, so a test over the scan's bound does not pay 850 synced
    /// appends.
    fn journal_of(name: &str, records: &[crate::audit::Record]) -> crate::audit::Journal {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        let path = crate::audit::journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        let mut bytes = Vec::with_capacity(records.len() * crate::audit::RECORD_LEN);
        for record in records {
            bytes.extend_from_slice(&record.image());
        }
        std::fs::write(&path, bytes).expect("write");
        let journal = crate::audit::Journal::at(&root);
        let built = journal.build_index().expect("the index builds");
        assert_eq!(built.records, records.len() as u64);
        journal
    }

    /// The same through a journal on disk, and onto `/pull/run.json`: the
    /// document of a server that has not pulled since it started carries the
    /// run the last process left open, and the calendar's horizon.
    #[test]
    fn a_restart_finds_the_open_run_in_the_journal_and_the_status_document_says_so() {
        let root = crate::scratch::path("pullrun-interrupted");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        let journal = crate::audit::Journal::at(&root);
        assert_eq!(restart_scan(&journal), Restart::Clean, "no journal, no run");
        let mut thread = journal
            .open_run(&opened_at(100, "kite", 20_000, 20_010))
            .expect("the opening record lands");
        for at in [110, 115] {
            journal
                .append_failure(&mut thread, &failure_at(at), "a gap")
                .expect("a failure lands");
        }
        let found = restart_scan(&journal);
        let run = found.interrupted().expect("the run is open");
        assert_eq!(
            (
                run.ordinal,
                run.failures,
                run.failures_whole,
                run.last_at_secs
            ),
            (0, 2, true, 115)
        );

        // 2026-12-02 is 29 days before the record's last day.
        let today = pull::calendar::SOURCED_LAST_DAY - 29;
        let doc = status_json(&Progress::default(), Some(today), Some(&found));
        let parsed: serde_json::Value = serde_json::from_str(&doc).expect("valid JSON");
        assert_eq!(parsed["running"], false);
        assert_eq!(parsed["restart"]["state"], "interrupted");
        assert_eq!(parsed["interrupted"]["ordinal"], 0);
        assert_eq!(parsed["interrupted"]["failures"], 2);
        assert_eq!(parsed["interrupted"]["failuresWhole"], true);
        assert_eq!(parsed["interrupted"]["from"], "2024-10-04");
        assert_eq!(parsed["interrupted"]["to"], "2024-10-14");
        assert_eq!(parsed["interrupted"]["lastRecordAtSecs"], 115);
        assert!(
            parsed["interrupted"]["words"]
                .as_str()
                .is_some_and(|words| words.starts_with("INTERRUPTED: a pull run")),
            "{doc}"
        );
        assert_eq!(parsed["calendar"]["recordTo"], "2026-12-31");
        assert_eq!(parsed["calendar"]["daysLeft"], 29);
        assert_eq!(parsed["calendar"]["endsSoon"], true);
        assert_eq!(parsed["calendar"]["ended"], false);
        assert_eq!(parsed["passCeiling"], MAX_PASSES);
        assert_eq!(parsed["stopReason"], serde_json::Value::Null);

        // Once the run is closed, nothing is open.
        journal
            .append(&closed_at(130, "kite", 20_000, 20_010))
            .expect("the closing record lands");
        let after = restart_scan(&journal);
        assert_eq!(after, Restart::Clean);
        let doc = status_json(&Progress::default(), None, Some(&after));
        let parsed: serde_json::Value = serde_json::from_str(&doc).expect("valid JSON");
        assert_eq!(parsed["interrupted"], serde_json::Value::Null);
        assert_eq!(parsed["restart"]["state"], "clean");
        assert_eq!(parsed["calendar"], serde_json::Value::Null);
        // A run of this process is shown: the scan is not this document's.
        let doc = status_json(&Progress::default(), None, None);
        let parsed: serde_json::Value = serde_json::from_str(&doc).expect("valid JSON");
        assert_eq!(parsed["restart"], serde_json::Value::Null);
    }

    /// **AFTER A RESTART MID-PULL, `/pull/run.json` IS NOT "NEVER RAN"**
    /// (DB-B34). The store's journal holds a run opened and never closed; a
    /// site loaded over it carries that run, and the route's document for a
    /// process that has not pulled names it. Before, the document was the
    /// 118-byte default, byte for byte what a fresh store answers.
    #[tokio::test]
    async fn the_status_route_of_a_restarted_server_names_the_run_it_interrupted() {
        let store = crate::scratch::path("pullrun-restart-store");
        let _ = std::fs::remove_dir_all(&store);
        std::fs::create_dir_all(store.join("manifest")).expect("mkdir");
        let masters = crate::scratch::path("pullrun-restart-masters");
        std::fs::create_dir_all(&masters).expect("mkdir");
        std::fs::write(
            masters.join("groww_instruments.csv"),
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n",
        )
        .expect("write");

        // A fresh store: nothing interrupted.
        let fresh = Loaded::new(Site::load(&masters, &store));
        assert_eq!(fresh.restart, Restart::Clean);
        let (_, _, body) = crate::server::pull_run_json(axum::extract::State(fresh)).await;
        let doc: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(doc["interrupted"], serde_json::Value::Null);
        assert_eq!(doc["restart"]["state"], "clean");
        assert_eq!(doc["calendar"]["recordTo"], "2026-12-31");

        // The previous process opened a run and died before closing it.
        let journal = crate::audit::Journal::at(&store);
        let mut thread = journal
            .open_run(&opened_at(1_791_000_000, "kite", 20_000, 20_010))
            .expect("the opening record lands");
        journal
            .append_failure(&mut thread, &failure_at(1_791_000_050), "a gap")
            .expect("a failure lands");
        let restarted = Loaded::new(Site::load(&masters, &store));
        assert!(restarted.restart.interrupted().is_some());
        let (status, _, body) =
            crate::server::pull_run_json(axum::extract::State(Loaded::clone(&restarted))).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let doc: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(doc["running"], false);
        assert_eq!(doc["interrupted"]["openedAtSecs"], 1_791_000_000_i64);
        assert_eq!(doc["interrupted"]["lastRecordAtSecs"], 1_791_000_050_i64);
        assert_eq!(doc["interrupted"]["failures"], 1);
        assert_eq!(doc["interrupted"]["source"], "kite");
        assert_eq!(doc["restart"]["state"], "interrupted");

        // Once a run is pressed in this process, the document is that run's.
        claim(&restarted);
        let (_, _, body) = crate::server::pull_run_json(axum::extract::State(restarted)).await;
        let doc: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(doc["interrupted"], serde_json::Value::Null);
        assert_eq!(doc["running"], true);
    }

    /// A synthetic census change, not an assertion that market bars exist.
    /// This exercises the real conductor's growth branch without retry sleeps.
    fn grow_synthetic_census(site: &Site) {
        use brutex_core::instrument::{Exchange, Segment};
        use brutex_core::symbol::Symbol;
        use brutex_core::vendor::Vendor;
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        use store::path::{Timeframe, YearMonth};

        let mut census = Manifest::open(Vendor::Zerodha, &[], &[]).expect("empty fixture");
        census
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: Symbol::new("NIFTY").expect("symbol"),
                    timeframe: Timeframe::MINUTE_1,
                    month: YearMonth::new(2025, 7).expect("month"),
                },
                rows: 1,
                first_ts_micros: 1,
                last_ts_micros: 1,
            })
            .expect("record counter");
        std::fs::write(
            manifest_path(&site.store_root, Vendor::Zerodha),
            census.image(),
        )
        .expect("write fixture census");
    }

    #[tokio::test]
    async fn a_growing_failed_pass_recovers_only_owed_legs_then_requires_full_idle_passes() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = std::sync::Arc::clone(&calls);
        let progress = simulated(
            "checkpoint-growth",
            vec![leg("zerodha", "1day"), leg("zerodha", "1min")],
            move |site, leg| {
                let mut seen = capture.lock().expect("calls");
                seen.push(leg.dir.clone());
                std::future::ready(if seen.len() == 2 {
                    assert_eq!(leg.dir, "1min");
                    grow_synthetic_census(&site);
                    receipt(503, "FAILED")
                } else {
                    receipt(200, "STORED")
                })
            },
        )
        .await;
        assert_eq!(
            *calls.lock().expect("calls"),
            [
                "1day", "1min", "1min", "1day", "1min", "1day", "1min", "1day", "1min"
            ]
        );
        assert_eq!(
            (
                progress.passes,
                progress.retries,
                progress.rows_at_start,
                progress.rows_now
            ),
            (5, 1, Some(0), Some(1))
        );
        assert_eq!(
            (
                progress.feeds[0].legs_done,
                progress.feeds[0].skipped,
                progress.feeds[0].retries
            ),
            (2, 0, 1)
        );
        assert!(!progress.running());
        let summary = progress.finished.expect("idle summary");
        assert!(
            summary.contains("Idle retries stopped")
                && summary.contains("Full basket coverage has not been verified")
        );
    }

    #[tokio::test]
    async fn empty_receipts_are_checkpointed_during_repairs_and_reasked_on_full_passes() {
        let empty = named_leg("zerodha", "1day", "empty");
        let owed = named_leg("zerodha", "1min", "owed");
        let mut passes = Passes::new("checkpoint-empty", vec![empty.clone(), owed.clone()]);
        assert_eq!(
            passes
                .respond(&[("empty", 200, "EMPTY"), ("owed", 503, "FAILED")])
                .await,
            vec![empty.clone(), owed.clone()]
        );
        assert_eq!(passes.respond(&[]).await, vec![owed.clone()]);
        assert_eq!(
            passes.respond(&[("empty", 200, "EMPTY")]).await,
            vec![empty, owed]
        );
        assert_eq!(passes.outcomes, [PassOutcome::Clean]);
    }

    #[tokio::test]
    async fn identical_partial_receipts_keep_the_full_failed_leg_owed_without_growth() {
        let daily = named_leg("zerodha", "1day", "daily");
        let minute = named_leg("zerodha", "1min", "from=2020-01-01&to=2026-08-31");
        let mut passes = Passes::new(
            "checkpoint-partial-limit",
            vec![daily.clone(), minute.clone()],
        );
        for index in 0..MAX_PASSES {
            let calls = passes.respond(&[(&minute.body, 200, "PARTIAL")]).await;
            assert_eq!(
                calls,
                if index == 0 {
                    vec![daily.clone(), minute.clone()]
                } else {
                    vec![minute.clone()]
                }
            );
            assert_eq!(passes.outcomes, [PassOutcome::Retry]);
            let progress = observed(&passes.site);
            assert_eq!(progress.retries, index);
            assert_eq!(progress.feeds[0].retries, index);
            assert_eq!(rows_now(&passes.site), Some(0));
        }
        let progress = observed(&passes.site);
        assert!(
            progress.feeds[0]
                .last_error
                .as_ref()
                .unwrap()
                .contains("HTTP 200")
        );
        let summary = summary_of(MAX_PASSES, progress.retries, Some(0), false);
        assert!(
            summary.contains("ceiling")
                && summary.contains("Full basket coverage has not been verified")
        );
    }

    #[tokio::test]
    async fn retries_count_rescheduled_feeds_and_reset_pass_counters() {
        let mut derivative = leg("zerodha", "options");
        derivative.route = Route::Fno;
        let (held, groups) = pass_fixture(
            "retry-counters",
            vec![leg("zerodha", "1day"), derivative, leg("groww", "1day")],
        );
        let mut outcomes = vec![PassOutcome::Clean; 2];
        let checkpoints = checkpoints_for(&groups);
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, leg: Leg| {
                std::future::ready(if leg.vendor == "zerodha" {
                    receipt(409, "NOT STARTED")
                } else {
                    receipt(200, "STORED")
                })
            },
        )
        .await;
        assert_eq!(outcomes, [PassOutcome::Retry, PassOutcome::Clean]);
        let first = observed(&held);
        assert_eq!(first.retries, 0, "a failure is not yet a retry");
        assert_eq!((first.feeds[0].legs_done, first.feeds[0].skipped), (1, 1));

        for _ in 0..2 {
            run_pass(
                &held,
                current(&held),
                &groups,
                &mut outcomes,
                &checkpoints,
                &|_site, _leg| std::future::ready(receipt(200, "STORED")),
            )
            .await;
        }
        let after = observed(&held);
        assert_eq!(after.retries, 1);
        assert_eq!((after.feeds[0].retries, after.feeds[1].retries), (1, 0));
        assert_eq!((after.feeds[0].legs_done, after.feeds[0].skipped), (2, 0));
        assert_eq!(outcomes, [PassOutcome::Clean, PassOutcome::Clean]);
    }

    #[tokio::test]
    async fn a_terminal_refusal_replaces_an_earlier_transient_cause_and_stays_halted() {
        let (held, groups) = pass_fixture("terminal-after-retry", vec![leg("zerodha", "1day")]);
        let mut outcomes = vec![PassOutcome::Clean];
        let checkpoints = checkpoints_for(&groups);
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, _leg| std::future::ready(receipt(503, "NOT STARTED")),
        )
        .await;
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, _leg| std::future::ready(receipt(422, "NOT STARTED")),
        )
        .await;
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, _leg| {
                panic!("a terminal feed must never be requested again");
                #[allow(unreachable_code)]
                std::future::ready(receipt(200, "STORED"))
            },
        )
        .await;
        assert_eq!(outcomes, [PassOutcome::Halted]);
        let progress = observed(&held);
        assert_eq!((progress.retries, progress.feeds[0].retries), (1, 1));
        let why = progress.feeds[0]
            .last_error
            .as_deref()
            .expect("terminal cause");
        assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
        assert!(run_summary(&progress, &outcomes, Some(0)).contains("INCOMPLETE"));
    }

    #[tokio::test]
    async fn a_panicking_chain_keeps_its_feed_index_after_a_sibling_halts() {
        let (held, groups) = pass_fixture(
            "panic-after-halt",
            vec![
                leg("zerodha", "1day"),
                leg("groww", "1day"),
                leg("groww", "1min"),
                named_leg("groww", "options", "option"),
                leg("dhan", "1day"),
            ],
        );
        let mut outcomes = vec![PassOutcome::Clean; 3];
        let checkpoints = checkpoints_for(&groups);
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, leg: Leg| {
                std::future::ready(if leg.vendor == "zerodha" {
                    receipt(401, "NOT STARTED")
                } else {
                    receipt(200, "STORED")
                })
            },
        )
        .await;
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, leg: Leg| async move {
                assert_ne!(leg.vendor, "zerodha", "the halted feed cannot be spawned");
                assert_ne!(leg.dir, "1min", "synthetic second-leg panic");
                receipt(200, "STORED")
            },
        )
        .await;
        assert_eq!(
            outcomes,
            [PassOutcome::Halted, PassOutcome::Retry, PassOutcome::Clean]
        );
        let progress = observed(&held);
        assert!(progress.feeds[0].credential_dead);
        assert_eq!(
            progress.feeds[1].legs_done, 1,
            "the panicked response never returned"
        );
        assert!(
            progress.feeds[1]
                .last_error
                .as_deref()
                .expect("panic cause")
                .contains("abnormally")
        );
        assert!(progress.feeds[2].last_error.is_none());
        assert_eq!(progress.feeds[2].legs_done, 1);
        assert_eq!(progress.feeds[1].skipped, 1, "the option never started");
        assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = std::sync::Arc::clone(&calls);
        run_pass(
            &held,
            current(&held),
            &groups,
            &mut outcomes,
            &checkpoints,
            &move |_site, leg: Leg| {
                capture.lock().expect("calls").push(leg);
                std::future::ready(receipt(200, "STORED"))
            },
        )
        .await;
        assert_eq!(
            *calls.lock().expect("calls"),
            [
                leg("groww", "1min"),
                named_leg("groww", "options", "option")
            ]
        );
        assert_eq!(
            outcomes,
            [PassOutcome::Halted, PassOutcome::Clean, PassOutcome::Clean]
        );
        let progress = observed(&held);
        assert_eq!(
            (
                progress.feeds[1].legs_done,
                progress.feeds[1].skipped,
                progress.feeds[1].retries
            ),
            (2, 1, 1)
        );
        assert_eq!(progress.retries, 1);
    }

    /// **A DEAD CREDENTIAL IS A REASON THIS PATH CAN NAME, AND HALT ON.**
    ///
    /// `autopilot::classify` had two production call sites, both inside
    /// `autopilot.rs`. The manual run — the one an operator presses — had no
    /// credential handling of any kind, so a dead token produced `retries`
    /// climbing and a `lastError` reading *"answered HTTP 401 … the leg is owed
    /// and will be asked for again"* for up to `MAX_PASSES` × `RETRY_WAIT`:
    /// **two hours of 401s against a token another system shares**, with the
    /// word *credential* appearing nowhere.
    ///
    /// §8 forbids minting one here, so there is nothing to retry INTO — the
    /// refreshed value is read on the next pull, and every request spent before
    /// then buys the same refusal.
    ///
    /// # What is asserted, and why each half is needed
    ///
    /// The classifier's own behaviour is proved in `autopilot`. What is pinned
    /// here is that the three vendor spellings this path will actually meet are
    /// classified as credential deaths rather than as transport blips — Kite's
    /// **403 `TokenException`**, which `docs/00-charter.md` §4z records as firing
    /// when a human merely logs into `kite.zerodha.com`, is the one that would
    /// otherwise silently burn a whole run.
    #[test]
    fn a_dead_token_is_classified_as_a_credential_and_not_as_a_blip() {
        use crate::autopilot::{Trouble, classify};

        for spelling in [
            "the broker credential could not be read: x",
            "no AWS identity: x",
            "the parameter path could not be built: x",
        ] {
            assert_eq!(
                classify(spelling),
                Trouble::Credential,
                "{spelling} must halt this feed, not be re-asked for two hours"
            );
        }
        // AND A TRANSPORT FAILURE IS STILL A TRANSPORT FAILURE. A classifier
        // that answered `Credential` to everything would halt a run on a blip,
        // which is the opposite defect and just as expensive.
        assert_eq!(
            classify("refused with status 500"),
            Trouble::Transport,
            "a 5xx is the vendor's own side and IS worth re-asking"
        );
    }

    /// **EVERY PAGE THIS ROUTE RENDERS CONTAINS THE WORD `credential`, AND THAT
    /// USED TO BE THE WHOLE TEST.**
    ///
    /// The test above proves `classify` on a REASON. This route was asking it
    /// about a PAGE, and the two are not the same input: `accepted_html` fills
    /// its headline from `server::halt_for`, whose only two possible answers
    /// both contain the word. So `classify(&html)` was `Credential` for every
    /// failing leg, and the feed was skipped for the rest of the run.
    ///
    /// Measured on 2026-08-25: Dhan answered HTTP 400 `DH-905 Input_Exception`
    /// with the credential read successfully fourteen times in the same run,
    /// and the page said `HALTED · CREDENTIAL DEAD`.
    ///
    /// The first two assertions are the bug. The rest are the cure not
    /// overshooting into the opposite defect — a page that shows a REAL dead
    /// token must still halt, or this repair costs two hours of 401s.
    #[test]
    fn the_headline_on_every_page_is_not_a_dead_token() {
        use crate::autopilot::{Trouble, classify, credential_fault_in_page};
        use crate::server::{HTTP_LIVE, HTTP_UNAVAILABLE, halt_for};

        for (name, prose) in [
            ("HTTP_LIVE", HTTP_LIVE),
            ("HTTP_UNAVAILABLE", HTTP_UNAVAILABLE),
        ] {
            // THE DEFECT, PINNED RATHER THAN DESCRIBED. `classify` still says
            // Credential here and that is CORRECT for its own contract -- the
            // fault was asking it at all. If this ever stops holding, the
            // paragraph was reworded, which is the repair that already failed
            // once and must not be mistaken for this one.
            assert_eq!(
                classify(prose),
                Trouble::Credential,
                "{name} contains the bare word, which is why a page may never reach `classify`"
            );
            assert!(
                !credential_fault_in_page(prose),
                "{name} is a paragraph about transport plumbing, not a dead token"
            );
        }

        // AND THE SAME THING THROUGH THE FUNCTION THAT ACTUALLY CHOOSES IT, so
        // a third sentence added to `halt_for` later is covered by this test
        // rather than by having been thought about.
        for broker in [crate::server::Broker::Live, crate::server::Broker::Refused] {
            assert!(
                !credential_fault_in_page(halt_for(broker)),
                "no halt_for sentence may read as a dead token: {broker:?}"
            );
        }

        // DHAN'S ACTUAL REFUSAL, from logs/events.ndjson on 2026-08-25, inside
        // the headline it was actually rendered under.
        let dhan = format!(
            "{HTTP_LIVE} the vendor refused with status 400 and named it: the \
             request as sent was rejected and will be rejected again unchanged. \
             It said: {{\"errorType\":\"Input_Exception\",\"errorCode\":\"DH-905\"}}"
        );
        assert!(
            !credential_fault_in_page(&dhan),
            "a malformed request is not a dead token, and calling it one hides \
             the bug that a token refresh can never fix"
        );

        // KITE'S EDGE 503, from the run at 2026-08-25T07:15 IST — the one that
        // came AFTER the token was refreshed, so the 403 was gone and this was
        // what was left. An HTML error page from a load balancer, not a Kite
        // API refusal: there is no `error_type` in it at all.
        //
        // This is the case that shows why the repair matters beyond one vendor.
        // A 503 is the vendor's own side failing, IS worth re-asking, and the
        // retry ladder handled it correctly — three attempts, then an honest
        // stop. Under the old page-sniffing test it would have been reported as
        // a dead credential and the feed skipped for the rest of the run,
        // turning a transient outage into a halt only a token refresh appears
        // to fix, which it cannot.
        let kite_503 = format!(
            "{HTTP_LIVE} the vendor refused with status 503: <html><body>\
             <h1>503 Service Unavailable</h1> No server is available to handle \
             this request. </body></html> — and its own side has now failed 3 \
             time(s) on this chunk, out of 3 allowed."
        );
        assert!(
            !credential_fault_in_page(&kite_503),
            "a 503 is the vendor's own side and IS worth re-asking; calling it \
             a dead token turns an outage into a halt"
        );

        // THE CURE MUST NOT OVERSHOOT. A real dead token still halts.
        for real in [
            "the vendor refused with status 403 and named it: TokenException",
            "the vendor refused with status 401",
            "Invalid_Authentication",
            "access token expired",
        ] {
            assert!(
                credential_fault_in_page(&format!("{HTTP_LIVE} {real}")),
                "a genuine auth refusal must still halt this feed: {real}"
            );
        }
    }

    /// **THE STATUS DOCUMENT SAYS WHICH FEED HALTED, AND WHY.**
    ///
    /// Without `credentialDead` on the wire a halted feed and a finished one are
    /// the same two booleans, and the operator is left inferring a credential
    /// death from a retry counter that has stopped moving. The field exists so
    /// the page can say it outright while the other feeds keep going — which is
    /// the whole point of halting per feed rather than per run.
    #[test]
    fn a_halted_feed_is_named_on_the_status_document_while_its_siblings_run() {
        let progress = Progress {
            passes: 3,
            retries: 9,
            rows_at_start: Some(0),
            rows_now: Some(500),
            stopping: false,
            finished: None,
            started: true,
            generation: 0,
            started_at_ms: None,
            asked: Vec::new(),
            feeds: vec![
                FeedReport {
                    vendor: "dhan".to_owned(),
                    legs: 2,
                    legs_done: 1,
                    doing: String::new(),
                    retries: 0,
                    last_error: Some("the reason is the CREDENTIAL".to_owned()),
                    finished: true,
                    credential_dead: true,
                    skipped: 0,
                    standing: 0,
                    store_unavailable: false,
                },
                FeedReport {
                    vendor: "groww".to_owned(),
                    legs: 2,
                    legs_done: 1,
                    doing: "Spot · 1 day".to_owned(),
                    retries: 0,
                    last_error: None,
                    finished: false,
                    credential_dead: false,
                    skipped: 0,
                    standing: 0,
                    store_unavailable: false,
                },
            ],
            stop: None,
        };
        let doc = progress.json();
        assert!(
            doc.contains("\"credentialDead\":true"),
            "the halted feed is named as halted: {doc}"
        );
        assert!(
            doc.contains("\"credentialDead\":false"),
            "and the healthy one is not — a document where every feed reads the \
             same is a document that distinguishes nothing: {doc}"
        );
        // THE RUN IS STILL RUNNING. Halting one feed must not end the press:
        // a credential is per vendor, and Groww has work left.
        assert!(
            doc.contains("\"running\":true"),
            "one dead token must not stop the feeds that still have a live one: {doc}"
        );
    }

    /// **A FEED WHOSE SPOT LEG FAILED DOES NOT SPEND REQUESTS ON ITS
    /// DERIVATIVES.**
    ///
    /// `by_feed` has always sorted legs by `ladder_rank` — `1day, 1min, 1s,
    /// futures, options` — so the ORDER was enforced from the start. The
    /// dependency that order exists to express was not: a failed spot leg set
    /// `failed = true` and the loop went straight on to `futures`, which is the
    /// half `pull::fold::Ladder` documents as *"only a completely clean stage
    /// advances"* and which nothing implemented.
    ///
    /// The cost of the gap is not abstract. An expired option's implied
    /// volatility is solved against the underlying's bar at the same minute —
    /// `pull::pricing`'s `NoSpotAtStamp` is that refusal by name — so every
    /// derivative request behind a failed spot leg was spent against a token
    /// **another system shares** to be told what the chain already knew.
    ///
    /// The fixture makes spot fail the cheapest honest way: a folder that is
    /// not there. No socket opens on either leg, so a derivative leg that ran
    /// anyway would fail for reasons of its own and a test asserting only the
    /// outcome would pass either way. The assertion is therefore on the REASON
    /// reaching the page.
    ///
    /// # Why this drives `run_chain` and not `conduct`
    ///
    /// This exercises the real local-archive handler in one pass. Its refusal
    /// leaves the derivative behind it unattempted. Retry counters and HTTP 409
    /// dependency skips are tested separately through `run_pass`.
    #[tokio::test]
    async fn a_failed_spot_leg_stops_the_derivative_legs_behind_it() {
        let site = site("spotfirst");
        claim(&site);

        let missing = crate::scratch::path("pullrun-no-such-folder");
        let _ = std::fs::remove_dir_all(&missing);

        // ONE FEED REPORT, because `run_chain` writes into `feeds[nth]` and
        // `conduct` is what normally builds that list.
        edit_slot(&site, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: pull::vendor::Feed::TrueData.wire().to_owned(),
                legs: 2,
                ..FeedReport::default()
            }];
        });

        let bound = core::time::Duration::from_secs(30);
        let ran = tokio::time::timeout(
            bound,
            run_chain(
                Loaded::clone(&site),
                current(&site),
                0,
                vec![
                    Leg {
                        route: Route::Spot,
                        vendor: pull::vendor::Feed::TrueData.wire().to_owned(),
                        dir: "1min".to_owned(),
                        label: "archive · 1 minute".to_owned(),
                        body: format!(
                            "target=swept&vendor={}&from=2025-07-01&to=2025-07-01&folder={}",
                            pull::vendor::Feed::TrueData.wire(),
                            missing.display()
                        ),
                    },
                    Leg {
                        route: Route::Fno,
                        vendor: pull::vendor::Feed::TrueData.wire().to_owned(),
                        dir: "options".to_owned(),
                        label: "expired options".to_owned(),
                        body: "underlying=NIFTY&series=opt&vendor=truedata\
                               &from=2025-07-01&to=2025-07-01"
                            .to_owned(),
                    },
                ],
                [OWED, OWED]
                    .into_iter()
                    .map(std::sync::atomic::AtomicU8::new)
                    .collect(),
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                request_leg,
            ),
        )
        .await;
        assert!(
            ran.is_ok(),
            "one pass over two legs, neither of which opens a socket, did not \
             finish inside {bound:?}"
        );

        let progress = observed(&site);
        let feed = progress
            .feeds
            .first()
            .expect("the press reports the one feed it was given");
        assert_eq!(
            feed.skipped, 1,
            "the options leg must be DEFERRED and counted, not attempted: a \
             request not made leaves no trace, so this counter is the only \
             thing that distinguishes a deferred leg from one that was never \
             asked for"
        );
        assert_eq!(feed.legs_done, 1, "only the spot response returned");

        // THE CAUSE IS THE SPOT FAILURE, NOT THE SKIP. `last_error` keeps the
        // FIRST reason deliberately, so an operator reads what went wrong
        // rather than what happened last. Asserted here because a later change
        // that overwrote it would still leave `skipped` at 1 and look correct.
        let why = feed.last_error.clone().unwrap_or_default();
        assert!(
            why.contains("archive · 1 minute"),
            "the reported cause must be the spot leg that actually failed: {why}"
        );

        // AND IT REACHES THE PAGE. A counter no document carries is a counter
        // the operator cannot read.
        let doc = observed(&site).json();
        assert!(
            doc.contains("\"skipped\":1"),
            "the deferral must be on the wire, or a feed that deferred half its \
             legs reads like one that had half as many: {doc}"
        );
    }
}
