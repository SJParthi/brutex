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
    /// The bars this run's OWN legs wrote, summed from each leg's receipt
    /// ([`bars_written_in`]); `None` once a STORED receipt stated no count
    /// or the sum did not fit, so the figure is unknown rather than short.
    ///
    /// # Why not the census (DB-B33, the third check's follow-up 10)
    ///
    /// The growth test and the "bars added" figure read the census of the
    /// run's own store vendors. Another writer of the SAME vendor (a `cli`
    /// pull, a second run over another window) moved that count too, so its
    /// rows kept this run alive, skipped [`RETRY_WAIT`] and were credited to
    /// it. A leg's receipt counts only what that leg wrote for the keys it
    /// asked for, so this figure is the run's own and no other writer's. The
    /// census is still shown (`rowsAtStart`, `rowsNow`) as what the store
    /// holds, and is no longer read as progress.
    pub bars_written: Option<u64>,
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
        out.push_str(",\"barsWritten\":");
        out.push_str(
            &self
                .bars_written
                .map_or_else(|| "null".to_owned(), |bars| bars.to_string()),
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
    /// Which pull it was: the spot route's run, or an expired-F&O walk. The
    /// two are re-run from two different places (`/pull/spot`, `/pull/fno`),
    /// and a closing record of one scope never closes a run of the other.
    pub scope: crate::audit::Scope,
    /// What it asked the vendor for beyond its window and source, as its
    /// opening record names it ([`crate::audit::ask_of`]): the feed, the
    /// identity policy, the target, the instruments and the rung (an F&O
    /// walk: the feed, the series, the expiry and the rung). `None` when the
    /// opening record names none (written before D-4950) or only a
    /// restatement of it was read: such a run is never read as superseded.
    pub ask: Option<u64>,
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
    /// How many records the scan could not decode BEFORE the opening record
    /// (none of them can be this run's close, so it is open whatever they
    /// were), and those records named ([`damaged_list`], bounded). They were
    /// on the ERROR line only, so `/pull/run.json` reported the run without
    /// its doubt (DB-B34, repair 3).
    pub undecodable_before: u64,
    /// [`Self::undecodable_before`]'s records, named; empty for none.
    pub undecodable: String,
    /// Whether a run NEWER than this one may also have been left open: one
    /// of the [`Self::undecodable_before`] records lies before a closing
    /// record the walk could not pair by position (it does not directly
    /// follow a record of the run it closed, so the walk took the newest
    /// open run of its scope, window and source), so that undecodable record
    /// may be the opening record the close truly belongs to, and the run the
    /// walk closed with it would then be open (DB-B34, repairs 4 and 6; the
    /// third check's probe `v62r3_p2`). This run is open either way; which
    /// run is the newest one open is then not known.
    pub newer_may_be_open: bool,
    /// Why the run records before the scan could not be read through the
    /// journal's index, when they could not: the run is still the newest one
    /// open (its opening record and everything after it were read), but an
    /// OLDER run left open would not have been seen (DB-B34, repair 3).
    pub older_unread: Option<String>,
    /// How many runs the records read leave open, this one included: **1**
    /// when it is the only one (AG-01, code-3). The walk kept every run left
    /// open and reported only the newest, so a second feed cut short in the
    /// same crash, or an older run of another ask cut short by an earlier
    /// crash, was on no document and no log line. `0` only on a run the walk
    /// has not finished with.
    pub open_count: u64,
    /// Every OTHER run the records read leave open, in the order their
    /// records were read, newest first; at most [`OPEN_NAMED`] `- 1` of them,
    /// and [`Self::open_count`] counts the rest (AG-01, code-3).
    pub also_open: Vec<AlsoOpen>,
    /// Run records the journal's index names before the scan that the scan
    /// did NOT read, because they lie behind the [`INTERRUPTED_RUNS`] newest
    /// ones it read: a run left open among them is not reported, and the
    /// document says how many there are (AG-01, code-3). `0` when every run
    /// record before the scan was read, or when the journal is no longer than
    /// the scan.
    pub runs_unread: u64,
}

/// How many runs left open one document names: **64**, the newest and 63
/// others ([`Interrupted::also_open`]), the same bound as the run records
/// read through the index ([`INTERRUPTED_RUNS`]). The records read can leave
/// more open (every one of the newest [`INTERRUPTED_SCAN`] records may be an
/// opening record); those are counted ([`Interrupted::open_count`]) and not
/// named, so the document stays bounded (AG-01).
pub const OPEN_NAMED: usize = INTERRUPTED_RUNS;

/// Another run the records read leave open, beside the one
/// [`Restart::Interrupted`] reports first (AG-01, code-3): what an operator
/// needs to re-run it, and no more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsoOpen {
    /// Its opening record's ordinal.
    pub ordinal: u64,
    /// When it was opened.
    pub opened_at_secs: i64,
    /// The newest record of it that was read.
    pub last_at_secs: i64,
    /// What it was fetching from.
    pub source: String,
    /// The spot route's run, or an expired-F&O walk.
    pub scope: crate::audit::Scope,
    /// Its ask, when its opening record was read and names one.
    pub ask: Option<u64>,
    /// Its window, as days since the epoch.
    pub window: Option<(u32, u32)>,
    /// Its member failures among the records read.
    pub failures: u64,
    /// Whether [`Self::failures`] is its whole count.
    pub failures_whole: bool,
    /// Why it may have been closed after all, when it may: a record after
    /// its opening record would not decode (it may be the closing record),
    /// or it is known only through a restated head and the run records before
    /// the scan could not be read (its closing record may lie there). `None`
    /// when the records read leave it open and nothing read could close it.
    pub doubt: Option<&'static str>,
}

impl AlsoOpen {
    /// The run in a clause: `record N (spot, from..to, source "S", ask X,
    /// last record at T s since the epoch, F member failure(s))`.
    fn clause(&self) -> String {
        let scope = match self.scope {
            crate::audit::Scope::Spot => "a pull run",
            crate::audit::Scope::Fno => "an expired-F&O walk",
        };
        let window = self.window.map_or_else(
            || "no window".to_owned(),
            |(from, to)| format!("{}..{}", day_text(from), day_text(to)),
        );
        let ask = self
            .ask
            .map_or_else(|| "no ask".to_owned(), |ask| format!("ask {ask:016x}"));
        let failures = if self.failures_whole {
            format!("{} member failure(s)", self.failures)
        } else {
            format!("at least {} member failure(s)", self.failures)
        };
        let doubt = self.doubt.map_or_else(String::new, |why| {
            format!("; it may have been closed: {why}")
        });
        format!(
            "record {} ({scope}, {window}, source {:?}, {ask}, opened at {} s and last read at \
             {} s since the epoch, {failures}{doubt})",
            self.ordinal, self.source, self.opened_at_secs, self.last_at_secs
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
        out.push_str(",\"scope\":");
        out.push_str(match self.scope {
            crate::audit::Scope::Spot => "\"spot\"",
            crate::audit::Scope::Fno => "\"fno\"",
        });
        out.push_str(",\"ask\":");
        match self.ask {
            Some(ask) => out.push_str(&quote_for_json(&format!("{ask:016x}"))),
            None => out.push_str("null"),
        }
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
        out.push_str(",\"mayBeClosed\":");
        match self.doubt {
            Some(why) => out.push_str(&quote_for_json(why)),
            None => out.push_str("null"),
        }
        out.push('}');
    }
}

impl From<&Interrupted> for AlsoOpen {
    fn from(run: &Interrupted) -> Self {
        Self {
            ordinal: run.ordinal,
            opened_at_secs: run.opened_at_secs,
            last_at_secs: run.last_at_secs,
            source: run.source.clone(),
            scope: run.scope,
            ask: run.ask,
            window: run.window,
            failures: run.failures,
            failures_whole: run.failures_whole,
            doubt: None,
        }
    }
}

/// The feeds whose spot leg writes NO opening record: every feed whose
/// source is a local archive folder (`pull::vendor::SourceKind::Folder`),
/// read from the vendor table, not listed here (AG-01, code-2). Such a leg
/// lands through `server::local_answer`, which writes only its closing record
/// (`landed_answer(.., None)`), so one cut short leaves nothing for the
/// restart scan to read.
fn unjournalled_feeds() -> impl Iterator<Item = pull::vendor::Feed> {
    pull::vendor::Feed::ALL.into_iter().filter(|feed| {
        matches!(
            feed.descriptor().transport.kind(),
            pull::vendor::SourceKind::Folder
        )
    })
}

/// [`unjournalled_feeds`] in one sentence, said with every restart answer.
fn unjournalled_words() -> String {
    let names = unjournalled_feeds()
        .map(pull::vendor::Feed::display)
        .collect::<Vec<_>>();
    if names.is_empty() {
        return String::new();
    }
    format!(
        " A spot leg over an archive-folder feed ({}) writes no opening record, only its \
         closing record when it lands, so one cut short leaves nothing for this scan to read \
         and is never reported here, whatever the answer above; `/audit` lists the closing \
         records such legs wrote.",
        names.join(", ")
    )
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
            // NOT "the newest 800 records were counted": a filed fault before
            // the scan that is read again and decodes is counted too, so the
            // count could exceed the scan it named (DB-B34, repair 3).
            format!(
                "at least {} member failure(s) (counted among the records read: the newest \
                 {INTERRUPTED_SCAN} and those read through the journal's index; older ones were \
                 not read)",
                self.failures
            )
        };
        let mut doubt = String::new();
        if self.undecodable_before > 0 {
            use core::fmt::Write as _;
            let _infallible = write!(
                doubt,
                " {} record(s) before its opening record could not be decoded ({}); none of \
                 them can be its closing record.",
                self.undecodable_before, self.undecodable
            );
        }
        if self.newer_may_be_open {
            doubt.push_str(
                " One of them may be the opening record that a later closing record belongs to \
                 (that closing record does not directly follow a record of the run the walk \
                 closed with it), so a run newer than this one may also have been left open; \
                 which run is the newest one open is not known, and this one is open either way.",
            );
        }
        if let Some(why) = &self.older_unread {
            use core::fmt::Write as _;
            let _infallible = write!(
                doubt,
                " The run records before the scan could not be read through the journal's \
                 index ({why}), so an older run left open would not have been seen."
            );
        }
        // WHERE IT IS RUN AGAIN, AND WHAT COUNTS AS AGAIN (DB-B34, repair 6).
        // The sentence told every reader to "press Pull", the spot route's
        // button, also of an expired-F&O walk (re-run from /pull/fno); and it
        // said "a later run of the same window and source" supersedes it, when
        // every Zerodha spot run of a window carries one source whatever its
        // target or rung. It is superseded only by a run whose opening record
        // names the same ask.
        let (what, again) = match self.scope {
            crate::audit::Scope::Spot => (
                "a pull run",
                "run the same pull again from /pull/spot (the same feed, target, instruments, \
                 rung and window)",
            ),
            crate::audit::Scope::Fno => (
                "an expired-F&O walk",
                "run the same walk again from /pull/fno (the same feed, underlying, series, \
                 expiry, rung and window)",
            ),
        };
        let superseded = match self.ask {
            Some(ask) => format!(
                "its opening record names its ask ({ask:016x}), and a later run of the same \
                 window and source whose opening record names the same ask, once it closes \
                 having asked the vendor (any close but NOT STARTED), supersedes this one"
            ),
            None => format!(
                "its opening record names no ask (it was written before this build, or only \
                 a restatement of it was read), so no later run can be shown to have made the \
                 same pull: it is reported until it lies behind the scan's newest \
                 {INTERRUPTED_SCAN} records and {INTERRUPTED_RUNS} run records"
            ),
        };
        let mut out = format!(
            "INTERRUPTED: {what}{window} was opened at journal record {} ({} s since the \
             epoch) and has no closing record; the last record of it that was read was written \
             at {} s since the epoch, with {failures} on record after it. No run of this server \
             process is in flight, so the process that ran it stopped before closing it, unless \
             another process (a `cli` pull, a second server) is still running it. Bars it had \
             already written are in the store; {again} to fetch what is left: {superseded}.\
             {doubt}",
            self.ordinal, self.opened_at_secs, self.last_at_secs
        );
        out.push_str(&self.others_words());
        out
    }

    /// The OTHER runs left open, and the run records the scan did not read,
    /// in sentences (AG-01, code-3); empty when this is the only run open and
    /// every run record before the scan was read, so the sentence of one run
    /// is byte for byte what it was.
    fn others_words(&self) -> String {
        use core::fmt::Write as _;
        let mut out = String::new();
        if self.open_count > 1 {
            let named = self
                .also_open
                .iter()
                .map(AlsoOpen::clause)
                .collect::<Vec<_>>()
                .join("; ");
            let _infallible = write!(
                out,
                " {} runs read have no closing record, not this one only: this is the one whose \
                 records were read last, and the others are {named}.",
                self.open_count
            );
            let unnamed = self
                .open_count
                .saturating_sub(1)
                .saturating_sub(self.also_open.len() as u64);
            if unnamed > 0 {
                let _infallible = write!(
                    out,
                    " {unnamed} more run(s) read have no closing record and are counted, not \
                     named (a document names {OPEN_NAMED}); `/audit` lists them."
                );
            }
            out.push_str(
                " Each was cut short, or is still being run by another process, as said of \
                 this one; each is re-run, and superseded, the same way.",
            );
        }
        if self.runs_unread > 0 {
            let _infallible = write!(
                out,
                " {} older run record(s) that the journal's index names lie before the \
                 {INTERRUPTED_RUNS} run records read through it and were not read, so a run left \
                 open among them is not reported.",
                self.runs_unread
            );
        }
        out
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
        out.push_str(",\"scope\":");
        out.push_str(match self.scope {
            crate::audit::Scope::Spot => "\"spot\"",
            crate::audit::Scope::Fno => "\"fno\"",
        });
        out.push_str(",\"ask\":");
        match self.ask {
            Some(ask) => out.push_str(&quote_for_json(&format!("{ask:016x}"))),
            None => out.push_str("null"),
        }
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
        out.push_str(",\"undecodableBefore\":");
        out.push_str(&self.undecodable_before.to_string());
        out.push_str(",\"undecodable\":");
        out.push_str(&quote_for_json(&self.undecodable));
        out.push_str(",\"newerMayBeOpen\":");
        out.push_str(if self.newer_may_be_open {
            "true"
        } else {
            "false"
        });
        out.push_str(",\"olderUnread\":");
        match &self.older_unread {
            Some(why) => out.push_str(&quote_for_json(why)),
            None => out.push_str("null"),
        }
        out.push_str(",\"words\":");
        out.push_str(&quote_for_json(&self.words()));
        // EVERY RUN LEFT OPEN (AG-01, code-3): after the keys above, which
        // are the newest run's exactly as before.
        out.push_str(",\"openCount\":");
        out.push_str(&self.open_count.to_string());
        out.push_str(",\"runRecordsUnread\":");
        out.push_str(&self.runs_unread.to_string());
        out.push_str(",\"alsoOpen\":[");
        for (at, other) in self.also_open.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            other.json(out);
        }
        out.push_str("]}");
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
    /// NOT KNOWN because a guess of the walk decides which runs are open
    /// (AG-01): the guesses, and the half that IS known -- every run the
    /// walk's reading leaves open, which of them no guess can move, and how
    /// many runs a guessed close surely left open. It was a bare
    /// [`Self::NotKnown`], which dropped every run the records surely leave
    /// open and wrote no ERROR line for any of them (second check, blocking 2).
    Guessed(Box<Guessed>),
}

/// [`Restart::Guessed`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guessed {
    /// The guesses, in words, bounded ([`guessed_words`]).
    pub why: String,
    /// The walk's own reading: the run read last that it leaves open,
    /// carrying every other one ([`Interrupted::also_open`]); `None` when it
    /// leaves none. A READING, not the answer: a guess may name the wrong run
    /// in it, so it is never served as `/pull/run.json`'s `interrupted`.
    pub reading: Option<Interrupted>,
    /// The opening ordinals of the runs a live guess names, ascending, at
    /// most [`OPEN_NAMED`]: each may be open or closed, whichever way that
    /// guess went.
    pub guessed_runs: Vec<u64>,
    /// The runs of the reading open WHICHEVER WAY THE GUESSES GO: no guess
    /// names them, and nothing read may have closed them (no record after
    /// their opening record that would not decode, no close of theirs before
    /// an unreadable index). Each is written to `/logs` as "a pull run has no
    /// closing record", as an interrupted answer's runs are.
    pub sure: Vec<AlsoOpen>,
    /// The runs of the reading that no guess names and that something read
    /// may have closed after all, each with why ([`AlsoOpen::doubt`]): a
    /// record after its opening record that would not decode, or (for a run
    /// known only through a restated head) a close of it that may lie before
    /// an unreadable index. Named, never logged as cut short and never
    /// dropped: every run of the reading is in exactly one of `sure`, this
    /// and `guessed_runs`, or counted in `unnamed` (third check of AG-01).
    pub doubted: Vec<AlsoOpen>,
    /// Runs of the reading counted and not named ([`OPEN_NAMED`]), so not
    /// sorted into sure or guessed.
    pub unnamed: u64,
    /// The pairing guesses whose count of runs left open is a fact
    /// ([`LeftOpen`]), at most [`OPEN_NAMED`].
    pub left_open: Vec<LeftOpen>,
}

impl Guessed {
    /// How many runs are surely open: every sure run, and every run a
    /// counted pairing surely left open.
    #[must_use]
    pub fn at_least_open(&self) -> u64 {
        (self.sure.len() as u64).saturating_add(
            self.left_open
                .iter()
                .map(|counted| counted.left_open)
                .fold(0, u64::saturating_add),
        )
    }

    /// The answer in sentences.
    fn words(&self) -> String {
        use core::fmt::Write as _;
        let at_least = self.at_least_open();
        let mut out = if at_least == 0 {
            format!("NOT KNOWN whether a pull run was left open: {}.", self.why)
        } else {
            format!(
                "NOT KNOWN which pull runs were left open, though at least {at_least} run(s) \
                 were: {}.",
                self.why
            )
        };
        if !self.sure.is_empty() {
            let named = self
                .sure
                .iter()
                .map(AlsoOpen::clause)
                .collect::<Vec<_>>()
                .join("; ");
            let _infallible = write!(
                out,
                " Open whichever way the guesses are read, with no closing record: {named}. Each \
                 was cut short, or is still being run by another process, and is re-run from \
                 /pull/spot (a pull run) or /pull/fno (an expired-F&O walk) with the same ask."
            );
        }
        if !self.doubted.is_empty() {
            let named = self
                .doubted
                .iter()
                .map(AlsoOpen::clause)
                .collect::<Vec<_>>()
                .join("; ");
            let _infallible = write!(
                out,
                " Open in the walk's reading and named by no guess, though something read may \
                 have closed it after all: {named}."
            );
        }
        for counted in &self.left_open {
            let _infallible = write!(
                out,
                " The closing record at record {} could end only one of the {} runs of its \
                 scope, window and source (source {:?}) opened at records {}, and no later \
                 closing record of theirs was read, so {} of them have no closing record; which \
                 is not known.",
                counted.close,
                counted.runs.len(),
                counted.source,
                counted
                    .runs
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                counted.left_open
            );
        }
        if !self.guessed_runs.is_empty() {
            let _infallible = write!(
                out,
                " A guess names the run(s) opened at records {}: each is open or closed \
                 whichever way its guess went.",
                self.guessed_runs
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if self.unnamed > 0 {
            let _infallible = write!(
                out,
                " {} more run(s) of the walk's reading have no closing record and are counted, \
                 not named (a document names {OPEN_NAMED}); `/audit` lists them.",
                self.unnamed
            );
        }
        // THE RUN RECORDS THE INDEX NAMES AND THE SCAN DID NOT READ, as an
        // interrupted answer says them: a guessed answer dropped the count.
        if let Some(behind) = self
            .reading
            .as_ref()
            .map(|run| run.runs_unread)
            .filter(|n| *n > 0)
        {
            let _infallible = write!(
                out,
                " {behind} older run record(s) that the journal's index names lie before the \
                 {INTERRUPTED_RUNS} run records read through it and were not read, so a run left \
                 open among them is not reported."
            );
        }
        out.push_str(" Nothing else is claimed; `/audit` lists the journal's runs.");
        out
    }

    /// The answer's own keys on `/pull/run.json`'s `restart` object, each
    /// after a comma.
    fn json(&self, out: &mut String) {
        out.push_str(",\"reading\":");
        match &self.reading {
            Some(run) => run.json(out),
            None => out.push_str("null"),
        }
        out.push_str(",\"guessedRuns\":[");
        out.push_str(
            &self
                .guessed_runs
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("],\"sureOpen\":[");
        for (at, run) in self.sure.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            run.json(out);
        }
        out.push_str("],\"doubtedOpen\":[");
        for (at, run) in self.doubted.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            run.json(out);
        }
        out.push_str("],\"leftOpen\":[");
        for (at, counted) in self.left_open.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            out.push_str("{\"close\":");
            out.push_str(&counted.close.to_string());
            out.push_str(",\"runs\":[");
            out.push_str(
                &counted
                    .runs
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push_str("],\"leftOpen\":");
            out.push_str(&counted.left_open.to_string());
            out.push_str(",\"scope\":");
            out.push_str(match counted.scope {
                crate::audit::Scope::Spot => "\"spot\"",
                crate::audit::Scope::Fno => "\"fno\"",
            });
            out.push_str(",\"source\":");
            out.push_str(&quote_for_json(&counted.source));
            out.push('}');
        }
        out.push_str("],\"atLeastOpen\":");
        out.push_str(&self.at_least_open().to_string());
        out.push_str(",\"unnamed\":");
        out.push_str(&self.unnamed.to_string());
    }
}

impl Restart {
    /// The run cut short, when one was found.
    #[must_use]
    pub const fn interrupted(&self) -> Option<&Interrupted> {
        match self {
            Self::Interrupted(run) => Some(run),
            Self::Clean | Self::NotKnown(_) | Self::Guessed(_) => None,
        }
    }

    /// The state's word on `/pull/run.json`.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Interrupted(_) => "interrupted",
            Self::NotKnown(_) | Self::Guessed(_) => "not_known",
        }
    }

    /// The state in one sentence, and what no state of the scan can see: a
    /// spot leg over an archive-folder feed, which writes no opening record
    /// ([`unjournalled_feeds`]; AG-01, code-2).
    #[must_use]
    pub fn words(&self) -> String {
        let mut out = self.state_words();
        out.push_str(&unjournalled_words());
        out
    }

    /// [`Self::words`] without the archive-folder sentence.
    fn state_words(&self) -> String {
        match self {
            // NOT "a later run of the same window and source closed (that run
            // asked the vendor for the same days again)" (DB-B34, repair 6):
            // every Zerodha spot run of a window carries one source, and a
            // refusal that asked for nothing closed runs it never touched.
            Self::Clean => format!(
                "No pull run was left open as far as the scan looked: every run record it read \
                 is closed, or it read none, and every record it read could be decoded. A run \
                 that was cut short counts as closed only once a later run of the same scope, \
                 window and source, whose opening record names the same ask (the feed, the \
                 identity policy, the target, the instruments and the rung), closed having \
                 asked the vendor (any close but NOT STARTED); a refusal that opened no run \
                 closes nothing. It reads the journal's newest {INTERRUPTED_SCAN} records and, \
                 where the journal's index can be read, the {INTERRUPTED_RUNS} newest run \
                 records before them; a run left open further back than that is not reported."
            ),
            Self::Interrupted(run) => run.words(),
            Self::NotKnown(why) => format!(
                "NOT KNOWN whether a pull run was left open: {why}. Nothing is claimed either \
                 way; `/audit` lists the journal's runs."
            ),
            Self::Guessed(guessed) => guessed.words(),
        }
    }
}

/// A day since the epoch as `YYYY-MM-DD`, or the number when it is no date.
fn day_text(days: u32) -> String {
    pull::session::Day::from_days(days).map_or_else(|_| days.to_string(), |day| day.to_string())
}

/// The runs the records `read` leave open, if any: the one whose records
/// were read last, carrying every other one ([`Interrupted::also_open`],
/// [`Interrupted::open_count`]). `read` is journal records oldest first, as
/// `(ordinal, record)`: every run record from the oldest of them on, and
/// whichever member failures were read with them.
///
/// This is the walk's own reading. Where that reading is a GUESS -- a close
/// that could end more than one run, a restated head behind a close that may
/// be its own (AG-01, codes 4 and 7) -- the walk still takes one reading, so
/// this answers, but [`restart_scan`] answers NOT KNOWN and names the guess.
///
/// # The walk
///
/// One pass, oldest to newest.
///
/// * An opening record starts a run. A restated one ("continues the run
///   opened at record N", written when another writer appended in between)
///   rejoins the run it names; it carries the opening record's own source,
///   window and time, so it stands for an opening record that lies before
///   `read`. A restatement of a run a close read earlier ENDED is not a new
///   run (a failure whose first append was refused, offered again behind
///   the closing record). A restatement of a run that IS open rejoins it
///   whatever closed before it: an earlier run's closing record of the same
///   window and source says nothing about this one.
/// * A restatement of a run NOT read, behind a closing record of its scope,
///   window and source that ended no run read, OPENS the run, and the pair
///   is a guess (AG-01, code-7): that close may be the run's own (the
///   legitimate late offer above, whose opening record lies before `read`)
///   or another run's, and neither record names an ask, so the two cannot
///   be told apart. It was dropped as closed, and a run cut short behind
///   another run's close read "clean"; and the close was never used up, so
///   800 restatements behind it were all dropped. Now only a run's FIRST
///   head read meets the close (every later head rejoins the run it
///   opened), so one run behind one close is one guess with every failure
///   counted; a head of another run of the same scope, window and source
///   meets the same close too, since it may be that run's close as well.
/// * A closing run record ends a run of its own scope, window AND source,
///   and no other (DB-B34, repair 6; D-4950): the run whose record it
///   directly follows (the ordinal before it), when that run is one, since
///   a close written through its run's thread always follows a record of
///   that run (`audit::Journal::close_run`). Else, when exactly one open run
///   is of its scope, window and source, that run (a close written before
///   D-4950). Else, when several are (AG-01, code-4): the one whose ask the
///   close names, if it names one and one run has it; the newest, when every
///   one of them names one same ask and the close is not NOT STARTED (the
///   same pull finished, so whichever it ended, the others are superseded
///   as below); and otherwise the newest as a GUESS that supersedes nothing
///   -- a close written without its run's thread could end either run. It
///   went silently to the newest, and two walks of one underlying and
///   window on two feeds read crosswise. A close that matches no open run
///   closes NOTHING. It was paired by its window alone too, and a press
///   refused before its run opened (the pull order, a mapping refusal, a
///   refused broker) wrote a NOT STARTED record under its target's label
///   with the window it asked for: that record closed the run cut short, of
///   any scope, and the next start answered "clean". A record of
///   [`crate::audit::Outcome::Refused`] closes nothing either: it is a
///   request refused before anything ran, which is how such a press is
///   written now.
/// * A closing record that ends a run it is PROVEN to end (by position, by
///   being the only candidate, by ask) SUPERSEDES every older run still open
///   of the same scope, window and source whose opening record names the
///   SAME ASK as the run it ended ([`crate::audit::ask_of`]): the same pull
///   (the feed, the identity policy, the target, the instruments, the rung)
///   was made again and finished, so the older run's work is done (DB-B34,
///   repairs 3 and 6). The source alone is not that evidence: it is the
///   policy and the endpoint TEMPLATE, the same for every target and rung,
///   cut to 64 bytes, so a 1day pull of the indices superseded a 1min pull
///   of the stocks over the same window. A run whose opening record names
///   no ask is never superseded and supersedes nothing, a close NOT STARTED
///   supersedes nothing: it asked the vendor for nothing (repair 5), and a
///   close paired by a guess supersedes nothing (AG-01: two Dhan walks cut
///   short read "clean" behind a Zerodha walk's close). Two runs of ONE ask
///   in flight at once (a server run and a `cli` pull, or two api
///   processes) whose close does not directly follow its run still cannot
///   be told apart: the walk ends the newest and supersedes the older, so if
///   the close was truly the OLDER run's, the newer one cut short reads as
///   superseded although only an older run is ever superseded. The same pull
///   finished either way, so its days were fetched; what is not said is that
///   the other run was cut short (`docs/06-limits.md`, D-4976).
/// * A member failure is counted to the run whose record it follows, and
///   NEVER opens one. Failures with no opening record before them are a
///   closed run's (the layout before D-3341, and `RunLedger::rest`'s when
///   the opening record was refused) or another run's; either way they are
///   no evidence that a run is open.
///
/// What is still open at the end was never closed. A guess still decides
/// the answer at the end only while a run it could have left open IS open:
/// a pairing guess whose other candidates all ended later by closes of their
/// own (not by being superseded), or a restated run that closed later,
/// decides nothing ([`Walk::guessed`]).
///
/// # Cost
///
/// O(records read x runs open): one pass, and each close looks at the open
/// list once. Both are bounded by the read ([`INTERRUPTED_SCAN`] records and
/// [`INTERRUPTED_RUNS`] run records), whatever the journal's length.
#[must_use]
pub fn interrupted_in(read: &[(u64, crate::audit::Record)]) -> Option<Interrupted> {
    walk(read).open
}

/// What one walk of [`interrupted_in`] found.
struct Walk {
    /// The run left open whose records were read last, carrying every other
    /// one left open; `None` when none is.
    open: Option<Interrupted>,
    /// The ordinal of the newest closing record the walk paired with a run
    /// it does NOT directly follow. A record before it that would not decode
    /// may be the opening record of another run of that window and source
    /// that the close truly belongs to, and the run the walk closed would
    /// then be open ([`Interrupted::newer_may_be_open`]). A close that
    /// directly follows a record of its run cannot be moved by such a record
    /// (nothing lies between them), and one that closed nothing hides no
    /// run.
    weak_close: Option<u64>,
    /// The walk's guesses that still decide the answer (AG-01, codes 4 and
    /// 7): while one is here the answer is NOT KNOWN, never "clean" and
    /// never a run named as the one cut short when it may be the wrong one.
    guessed: Vec<Guess>,
    /// The live pairing guesses whose count of runs left open is a fact of
    /// the records ([`LeftOpen`]); the last conditions on it (damage, an
    /// unreadable index) are [`judged`]'s.
    left_open: Vec<LeftOpen>,
}

/// A record's window, as days since the epoch; `None` for a record that
/// names none.
fn window_of(record: &crate::audit::Record) -> Option<(u32, u32)> {
    (record.from_days != 0 || record.to_days != 0).then_some((record.from_days, record.to_days))
}

/// Whether `record` is a closing record: a run record that neither opens a
/// run nor refuses one.
fn is_close(record: &crate::audit::Record) -> bool {
    use crate::audit::{Kind, Outcome};
    record.kind == Kind::Run && !matches!(record.outcome, Outcome::Opened | Outcome::Refused)
}

/// Whether two records are of one scope, window and source.
fn same_key(one: &crate::audit::Record, other: &crate::audit::Record) -> bool {
    one.scope == other.scope && window_of(one) == window_of(other) && one.source == other.source
}

/// A reading the walk had to take that nothing read proves (AG-01).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Guess {
    /// A closing record at `close` that does not directly follow a record of
    /// the run it closes, read while the runs opened at `runs` (newest first;
    /// the walk paired it with the first) were open with its scope, window
    /// and source and did not name one same ask (code-4). `at` is the
    /// close's position among the records walked.
    Pairing {
        close: u64,
        runs: Vec<u64>,
        at: usize,
    },
    /// A head at `head` restating the run opened at `run` (not read), behind
    /// the closing record at `close` of its scope, window and source that
    /// ended no run read (code-7). `at` is the head's position among the
    /// records walked.
    Unseen {
        close: u64,
        head: u64,
        run: u64,
        at: usize,
    },
}

/// A live pairing guess whose close SURELY left all of its runs open but
/// one (AG-01, second check's blocking 2): a close ends at most one run, so of
/// the `runs.len()` runs of its scope, window and source open when it was
/// written, `runs.len() - 1` had no closing record after it. Made only where
/// that count is a fact of the records and not of the walk's reading: the
/// guess is the first of its scope, window and source, no closing record of
/// those follows it, no record that would not decode lies after the oldest of
/// its runs' opening records, and none of its runs is known only through a
/// restated head whose close may lie before an unreadable index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftOpen {
    /// The closing record the guess is about.
    pub close: u64,
    /// The opening ordinals of the runs it could end, newest first.
    pub runs: Vec<u64>,
    /// How many of them have no closing record: `runs.len() - 1`.
    pub left_open: u64,
    /// Their scope.
    pub scope: crate::audit::Scope,
    /// Their source.
    pub source: String,
}

impl Guess {
    /// Where among the records walked the record that made the guess lies.
    const fn at(&self) -> usize {
        match self {
            Self::Pairing { at, .. } | Self::Unseen { at, .. } => *at,
        }
    }

    /// The runs the guess names: every run a pairing could end, or the
    /// restated run behind an unseen close.
    fn names(&self) -> &[u64] {
        match self {
            Self::Pairing { runs, .. } => runs,
            Self::Unseen { run, .. } => core::slice::from_ref(run),
        }
    }

    /// The guess in a clause, for a NOT KNOWN answer.
    fn words(&self) -> String {
        match self {
            Self::Pairing { close, runs, .. } => format!(
                "the closing record at record {close} does not directly follow a record of the \
                 run it closes, and a close written without its run's thread could end either \
                 run: {} runs of its scope, window and source were open when it was written, \
                 opened at records {}, their opening records do not name one same ask and the \
                 close names none, so which of them it ended, and which was cut short, is not \
                 known",
                runs.len(),
                runs.iter()
                    .take(OPEN_NAMED)
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unseen {
                close, head, run, ..
            } => format!(
                "the run opened at record {run}, read only through its head restated at record \
                 {head}, has before it the closing record at record {close} of its scope, window \
                 and source, which ended no run read: that close may be this run's own (a \
                 failure offered again behind its run's close is restated so) or another run's, \
                 and neither record names an ask, so whether this run was cut short is not known"
            ),
        }
    }
}

/// A closing record that ended no run that was read: its ordinal, scope,
/// window and source, so a head restated after it is read as a guess.
type Unseen = (u64, crate::audit::Scope, Option<(u32, u32)>, String);

/// [`walk`]'s state between two records.
#[derive(Default)]
struct Walker {
    /// The runs opened and not yet closed, in the order they were first
    /// read.
    open: Vec<Interrupted>,
    /// Opening ordinals of the runs a closing record ended: a head restated
    /// after one is not a new run.
    closed: Vec<u64>,
    /// The closing records that ended no run that was read: a run's first
    /// restated head read behind one is a guess ([`Guess::Unseen`]).
    unseen: Vec<Unseen>,
    /// The run whose record was read last, while a member failure or a close
    /// may still follow it.
    current: Option<u64>,
    /// [`Walk::weak_close`].
    weak_close: Option<u64>,
    /// Every guess taken, live or not; [`walk`] keeps the live ones.
    guessed: Vec<Guess>,
    /// Opening ordinals of the runs a later run of the same ask superseded:
    /// done, but not ended by a close of their own.
    superseded: Vec<u64>,
    /// The position among the records walked of the record being read.
    position: usize,
}

/// [`interrupted_in`]'s walk, with the newest closing record it could not
/// pair by position ([`Walk::weak_close`]) and its live guesses.
fn walk(read: &[(u64, crate::audit::Record)]) -> Walk {
    use crate::audit::{Kind, Outcome};
    let mut walker = Walker::default();
    // The ordinal of the record read just before this one: a close written
    // through its run's thread directly follows a record of that run.
    let mut previous: Option<u64> = None;
    for (position, (ordinal, record)) in read.iter().enumerate() {
        walker.position = position;
        let window = window_of(record);
        let follows = previous.and_then(|before| before.checked_add(1)) == Some(*ordinal);
        previous = Some(*ordinal);
        match (record.kind, record.outcome) {
            (Kind::Run, Outcome::Opened) => walker.opened(*ordinal, record, window),
            (Kind::Run, Outcome::Refused) => walker.current = None,
            (Kind::Run, _) => walker.closed(*ordinal, record, window, follows),
            (Kind::MemberFailure, _) => {
                if let Some(run) = walker
                    .current
                    .and_then(|first| walker.open.iter_mut().find(|run| run.ordinal == first))
                {
                    run.failures = run.failures.saturating_add(1);
                    run.last_at_secs = run.last_at_secs.max(record.at_unix_secs);
                }
            }
        }
    }
    // A GUESS DECIDES THE ANSWER ONLY WHILE A RUN IT COULD HAVE LEFT OPEN IS
    // OPEN (AG-01): a pairing whose other candidates all ended later by a
    // close of their own, or a restated run that closed later, is settled
    // whichever way it went. A candidate SUPERSEDED later did not end by a
    // close of its own: the guessed close may have been its close, and the
    // run the walk closed with it is then open, so that guess still decides.
    let unsettled: std::collections::HashSet<u64> = walker
        .open
        .iter()
        .map(|run| run.ordinal)
        .chain(walker.superseded.iter().copied())
        .collect();
    let still_open: std::collections::HashSet<u64> =
        walker.open.iter().map(|run| run.ordinal).collect();
    let live = |guess: &Guess| match guess {
        Guess::Pairing { runs, .. } => runs.iter().skip(1).any(|other| unsettled.contains(other)),
        Guess::Unseen { run, .. } => still_open.contains(run),
    };
    // HOW MANY RUNS A LIVE PAIRING SURELY LEFT OPEN (AG-01, second check's
    // blocking 2). A close ends at most one run, so of the k runs it could
    // end, k - 1 had no closing record after it -- a fact of the records, not
    // of the walk's reading, while nothing before it was a guess about their
    // scope, window and source (the k would be the reading's) and nothing
    // after it is a close of theirs (one could end, or supersede, any of
    // them). Bounded: each live pairing looks at the records after it once.
    let mut left_open = Vec::new();
    for (index, guess) in walker.guessed.iter().enumerate() {
        let Guess::Pairing { close, runs, at } = guess else {
            continue;
        };
        let Some((_, made)) = read.get(*at) else {
            continue;
        };
        let first_of_key = walker.guessed.get(..index).is_some_and(|earlier| {
            earlier.iter().all(|other| {
                read.get(other.at())
                    .is_none_or(|(_, record)| !same_key(record, made))
            })
        });
        let last_close_of_key = read
            .get(at.saturating_add(1)..)
            .unwrap_or_default()
            .iter()
            .all(|(_, record)| !(is_close(record) && same_key(record, made)));
        if live(guess) && first_of_key && last_close_of_key {
            left_open.push(LeftOpen {
                close: *close,
                runs: runs.clone(),
                left_open: (runs.len() as u64).saturating_sub(1),
                scope: made.scope,
                source: made.source.clone(),
            });
        }
    }
    let guessed = walker
        .guessed
        .into_iter()
        .filter(|guess| live(guess))
        .collect();
    // EVERY RUN LEFT OPEN (AG-01, code-3). The newest still-open run was
    // reported and every other was dropped: a second feed cut short in the
    // same crash, or an older run of another ask cut short by an earlier
    // crash, was on no document and no log line.
    let open_count = walker.open.len() as u64;
    let mut runs = walker.open;
    let open = runs.pop().map(|mut newest| {
        newest.open_count = open_count;
        newest.also_open = runs
            .iter()
            .rev()
            .take(OPEN_NAMED.saturating_sub(1))
            .map(AlsoOpen::from)
            .collect();
        newest
    });
    Walk {
        open,
        weak_close: walker.weak_close,
        guessed,
        left_open,
    }
}

impl Walker {
    /// An opening record at `ordinal`, or a restatement of one.
    fn opened(&mut self, ordinal: u64, record: &crate::audit::Record, window: Option<(u32, u32)>) {
        let restated = record
            .note
            .strip_prefix("continues the run opened at record ")
            .and_then(|rest| rest.trim().parse::<u64>().ok());
        let first = restated.unwrap_or(ordinal);
        let known = self.open.iter().any(|run| run.ordinal == first);
        if restated.is_some() && !known && self.closed.contains(&first) {
            self.current = None;
            return;
        }
        // A HEAD RESTATED BEHIND A CLOSE THAT ENDED NO RUN READ (AG-01,
        // code-7): the close may be this run's own or another run's, and
        // nothing read tells which. The run is OPENED and the pair is a
        // guess. Only the run's first head read meets the close: every later
        // head of it rejoins the open run (`known`), so 800 restatements
        // behind one close are one run, one guess, every failure counted.
        // The close stays there for a head of ANOTHER run, whose close it may
        // be just as well.
        if restated.is_some() && !known {
            let behind = self.unseen.iter().find(|(_, scope, held, source)| {
                *scope == record.scope && *held == window && *source == record.source
            });
            if let Some((close, ..)) = behind {
                self.guessed.push(Guess::Unseen {
                    close: *close,
                    head: ordinal,
                    run: first,
                    at: self.position,
                });
            }
        }
        if !known {
            self.open.push(Interrupted {
                ordinal: first,
                opened_at_secs: record.at_unix_secs,
                last_at_secs: record.at_unix_secs,
                source: record.source.clone(),
                scope: record.scope,
                // A restatement's note names the run it continues, not the
                // run's ask: the ask is the opening record's, and is not known
                // from a restatement.
                ask: if restated.is_some() {
                    None
                } else {
                    crate::audit::ask_of(&record.note)
                },
                window,
                failures: 0,
                failures_whole: restated.is_none(),
                note: if restated.is_some() {
                    String::new()
                } else {
                    record.note.clone()
                },
                undecodable_before: 0,
                undecodable: String::new(),
                newer_may_be_open: false,
                older_unread: None,
                open_count: 0,
                also_open: Vec::new(),
                runs_unread: 0,
            });
        }
        self.current = Some(first);
    }

    /// A closing record at `ordinal`; `follows` when the record read just
    /// before it is the one at the ordinal before it.
    fn closed(
        &mut self,
        ordinal: u64,
        record: &crate::audit::Record,
        window: Option<(u32, u32)>,
        follows: bool,
    ) {
        // ITS OWN SCOPE, WINDOW AND SOURCE, AND NOTHING ELSE (DB-B34, repair
        // 6). A close with no run of all three open closed the newest run of
        // its WINDOW: a press refused before its run opened (NOT STARTED
        // under its target's label) closed the run cut short, of either scope.
        let mine = |run: &Interrupted| {
            run.scope == record.scope && run.window == window && run.source == record.source
        };
        // A NOT STARTED CLOSE SUPERSEDES NOTHING (repair 5): it asked the
        // vendor for nothing (no credential, no budget, an archive feed), so
        // it is no evidence that the older run's days were fetched. It still
        // closes its own run.
        let not_started = record.outcome == crate::audit::Outcome::NotStarted;
        // THE RUN IT DIRECTLY FOLLOWS FIRST: a close written through its run's
        // thread always follows a record of that run, so it is never handed to
        // another run of the same pull.
        //
        // NOT AN EXPIRED-F&O WALK'S, WHILE TWO OR MORE OF ITS WALKS ARE OPEN
        // (AG-01, second check's blocking 1; the attack's A1). An F&O walk
        // opens and closes with a plain `Journal::append`, not through
        // `open_run`/`close_run` (D-4950 Decided 2 is not yet built for it:
        // server.rs, R4-02), so its close lands wherever the append lock let
        // it, and directly behind ANOTHER walk's opening record as easily as
        // behind its own. Walk G opens, walk D opens, G's close lands behind
        // D's opening record: by position the close was D's, G was reported
        // cut short and D, the walk that was, was hidden. With two or more
        // walks of its scope, window and source open, position proves nothing
        // and the close goes to the candidates' rules below (one ask, a named
        // ask, or a guess). With one, it is that walk's either way.
        let positional = record.scope != crate::audit::Scope::Fno
            || self.open.iter().filter(|run| mine(run)).nth(1).is_none();
        let beside = self
            .current
            .filter(|_| follows && positional)
            .and_then(|first| self.open.iter().position(|run| run.ordinal == first))
            .filter(|&at| self.open.get(at).is_some_and(&mine));
        let (ended, proven) = match beside {
            Some(at) => (Some(at), true),
            None => self.by_candidates(ordinal, record, not_started, &mine),
        };
        if beside.is_none() && ended.is_some() {
            self.weak_close = Some(ordinal);
        }
        self.current = None;
        let Some(at) = ended else {
            self.unseen
                .push((ordinal, record.scope, window, record.source.clone()));
            return;
        };
        let run = self.open.remove(at);
        // A LATER RUN OF THE SAME ASK THAT CLOSES SUPERSEDES AN OLDER ONE LEFT
        // OPEN (DB-B34, repairs 3 and 6). Nothing closes an interrupted run's
        // opening record, so after the operator made the same pull again and
        // it closed, every later start still answered INTERRUPTED. The closing
        // run asked the vendor for the same instruments at the same rung over
        // the same window -- its opening record names the same ask -- and a
        // pull fetches what the store does not hold, so the older run's work
        // is done. Append-only: nothing is written; the walk reads it so. The
        // window and source alone are no such evidence: every Zerodha spot run
        // of a window carries one source, whatever its target or rung.
        //
        // ONLY A CLOSE PROVEN TO BE THE RUN'S (AG-01): a close the walk paired
        // by a guess is no evidence the pull it guessed was made and finished,
        // and superseding by it read two Dhan walks cut short as "clean".
        if let Some(ask) = run.ask.filter(|_| proven && !not_started) {
            let closed = &mut self.closed;
            let superseded = &mut self.superseded;
            self.open.retain(|older| {
                let same = mine(older) && older.ask == Some(ask);
                if same {
                    closed.push(older.ordinal);
                    superseded.push(older.ordinal);
                }
                !same
            });
        }
        self.closed.push(run.ordinal);
    }

    /// A close that does not directly follow a record of an open run of its
    /// own: the position in [`Self::open`] of the run it ends, if any, and
    /// whether it is PROVEN to end that run (AG-01, code-4).
    ///
    /// * No open run of its scope, window and source: none.
    /// * Exactly one: that one (a close written before D-4950), proven.
    /// * Several: the one whose ask the close names, when it names one and
    ///   exactly one run has it (no build writes an ask on a close yet; a
    ///   later one may), proven. Else, when every one names one same ask and
    ///   the close is not NOT STARTED, the newest, proven: the same pull
    ///   finished, whichever run it was, so the others are superseded with
    ///   it. Else the newest as a GUESS ([`Guess::Pairing`]).
    fn by_candidates(
        &mut self,
        ordinal: u64,
        record: &crate::audit::Record,
        not_started: bool,
        mine: &dyn Fn(&Interrupted) -> bool,
    ) -> (Option<usize>, bool) {
        let candidates: Vec<usize> = self
            .open
            .iter()
            .enumerate()
            .filter(|(_, run)| mine(run))
            .map(|(at, _)| at)
            .collect();
        let (Some(&newest), true) = (candidates.last(), candidates.len() > 1) else {
            return (candidates.last().copied(), true);
        };
        let ask_at = |at: &usize| self.open.get(*at).and_then(|run| run.ask);
        if let Some(named) = crate::audit::ask_of(&record.note) {
            let mut by_ask = candidates.iter().filter(|at| ask_at(at) == Some(named));
            if let (Some(&one), None) = (by_ask.next(), by_ask.next()) {
                return (Some(one), true);
            }
        }
        let first_ask = candidates.first().and_then(ask_at);
        let one_ask = first_ask.is_some() && candidates.iter().all(|at| ask_at(at) == first_ask);
        if one_ask && !not_started {
            return (Some(newest), true);
        }
        let runs = candidates
            .iter()
            .rev()
            .filter_map(|at| self.open.get(*at).map(|run| run.ordinal))
            .collect();
        self.guessed.push(Guess::Pairing {
            close: ordinal,
            runs,
            at: self.position,
        });
        (Some(newest), false)
    }
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
/// the lock stayed held, an entry is unreadable) the answer is never a plain
/// "clean" (repair 3): with no run left open in the scan it is
/// [`Restart::NotKnown`] with both reasons, whatever run records the scan
/// holds, since an older run may be open behind them; a run the scan read
/// opened and unclosed is still reported, the reason on its document
/// ([`Interrupted::older_unread`]) and its ERROR line; a run known only
/// through a restated head is NOT KNOWN, since its close may lie before the
/// scan. It once answered "clean" whenever the scan held any opening or
/// closing record, and before that "INTERRUPTED ... The server stopped while
/// it ran" of a journal whose newest run record was a closing one.
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
/// that closing record). The one place it cannot decide is strictly before
/// the opening record of the run the readable records leave open; that run
/// is still reported, and the damage is named on its log line and its
/// document ([`Interrupted::undecodable_before`]). Such a record can still
/// move a LATER pairing not made by position, so when one follows it the
/// document says a newer run may be open as well
/// ([`Interrupted::newer_may_be_open`]).
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
    use crate::audit::Log;
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
                Err(fault) => damaged.push((entry.ordinal, fault.to_string().into())),
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
    let mut runs_unread = 0_u64;
    if scan_first > 0 {
        match older_runs_built(journal, records, scan_first, waits) {
            Ok(older) => {
                read = older.read;
                damaged.extend(older.damaged);
                runs_unread = older.runs_unread;
            }
            Err(why) => unread = Some(why),
        }
    }
    read.extend(tail);
    judged(
        &read,
        damaged,
        unread,
        Bounds {
            records,
            scan_first,
            runs_unread,
        },
    )
}

/// How far [`restart_scan`] read: the journal's length, the first record of
/// the scan, and the run records the index names before the scan that it
/// did not read ([`Interrupted::runs_unread`]).
#[derive(Clone, Copy)]
struct Bounds {
    records: u64,
    scan_first: u64,
    runs_unread: u64,
}

/// [`restart_scan`]'s answer over what it read: `read` oldest first,
/// `damaged` the records it could not decode, `unread` why the run records
/// before the scan could not be read through the index, when they could not.
/// The journal is not touched here, so the same answer is reached from
/// records in memory ([`restart_in`]).
fn judged(
    read: &[(u64, crate::audit::Record)],
    mut damaged: Vec<Damaged>,
    unread: Option<String>,
    bounds: Bounds,
) -> Restart {
    use crate::audit::{Kind, Outcome};
    damaged.sort_by_key(|(ordinal, _)| *ordinal);
    damaged.dedup_by_key(|(ordinal, _)| *ordinal);
    // A run record was read: an opening record, or a closing one. A refusal
    // is none. It no longer decides the answer (repair 3); it words it.
    let settled = read
        .iter()
        .any(|(_, record)| record.kind == Kind::Run && record.outcome != Outcome::Refused);
    let Walk {
        open: found,
        weak_close,
        guessed,
        left_open,
    } = walk(read);
    // WHICH OF THE DAMAGED RECORDS PUT THE ANSWER IN DOUBT. A record that
    // would not decode may be a run's opening record or its closing record.
    // STRICTLY before the opening record of the run the readable records
    // leave open it cannot be that run's close (a close follows its
    // opening), so that run IS open and the answer stands; the damage is
    // named on its log line and on the document. It may still be the
    // opening record a LATER close belongs to, where the walk paired that
    // close with the newest run of its scope, window and source rather than
    // with a run it directly follows: the run the walk closed with it,
    // possibly newer than the one reported, would then be open, and the
    // document says so ([`Interrupted::newer_may_be_open`], repairs 4 and 6).
    // Anywhere else -- AT that opening record (a run known only through a
    // restated head whose own first record is the damaged one: what that
    // opening record named, its ask among it, is not known; repair 3 made
    // this `<`, it was `<=`), after it, or
    // anywhere at all when the readable records leave no run open -- it may
    // be the record that decides, and the answer is NOT KNOWN.
    let opened = found.as_ref().map(|run| run.ordinal);
    let (before, doubt): (Vec<Damaged>, Vec<Damaged>) = damaged
        .into_iter()
        .partition(|(ordinal, _)| opened.is_some_and(|open| *ordinal < open));
    if !doubt.is_empty() {
        return not_known(damaged_words(&doubt, found.as_ref()));
    }
    // A GUESS THAT STILL DECIDES IS NOT AN ANSWER (AG-01, codes 4 and 7): a
    // close that could end either of two runs, or a restated run behind a
    // close that may be its own. The walk took one reading so that it could
    // go on; the answer names the guess instead of the reading -- and,
    // beside it, every run of the reading, the ones no guess can move, and
    // how many runs a guessed close surely left open (second check's
    // blocking 2: they were dropped, with every ERROR line).
    let span = Span {
        records: bounds.records,
        scan_first: bounds.scan_first,
        settled,
        weak_close,
        runs_unread: bounds.runs_unread,
    };
    if !guessed.is_empty() {
        return guessed_answer(&guessed, found, left_open, &before, unread.as_deref(), span);
    }
    verdict(found, &before, unread, span)
}

/// The scan's answer over records already read, oldest first, with nothing
/// damaged and every run record before them read: [`restart_scan`] without
/// the journal, for a test that states a journal's shape as records.
#[cfg(test)]
pub(crate) fn restart_in(read: &[(u64, crate::audit::Record)]) -> Restart {
    let records = read
        .last()
        .map_or(0, |(ordinal, _)| ordinal.saturating_add(1));
    judged(
        read,
        Vec::new(),
        None,
        Bounds {
            records,
            scan_first: 0,
            runs_unread: 0,
        },
    )
}

/// Why the answer is NOT KNOWN when guesses decide it: each guess, bounded,
/// and what the walk's own reading would have said, so the half that is
/// known is told and not handed over as the answer (AG-01).
fn guessed_words(guessed: &[Guess], found: Option<&Interrupted>, unread: Option<&str>) -> String {
    use core::fmt::Write as _;
    let mut out = guessed
        .iter()
        .take(DAMAGED_NAMED)
        .map(Guess::words)
        .collect::<Vec<_>>()
        .join("; and ");
    if let Some(more) = guessed.len().checked_sub(DAMAGED_NAMED).filter(|n| *n > 0) {
        let _infallible = write!(out, "; and {more} more such guess(es)");
    }
    match found {
        Some(run) => {
            let _infallible = write!(
                out,
                ". Read one way, the records leave {} run(s) open, the run opened at record {} \
                 read last; read the other way, another run is open in its place or as well",
                run.open_count.max(1),
                run.ordinal
            );
        }
        None => out.push_str(
            ". Read one way, the records leave no run open; read the other way, a run is open",
        ),
    }
    if let Some(why) = unread {
        let _infallible = write!(
            out,
            ". The run records before the scan could not be read through the journal's index \
             ({why}) either"
        );
    }
    out
}

/// How much of the journal [`restart_scan`] read: its length, the first
/// record of the scan, whether the records read hold a run record (an
/// opening or a closing one), the newest closing record the walk could not
/// pair by position ([`Walk::weak_close`]), and the run records before the
/// scan it did not read ([`Interrupted::runs_unread`]).
#[derive(Clone, Copy)]
struct Span {
    records: u64,
    scan_first: u64,
    settled: bool,
    weak_close: Option<u64>,
    runs_unread: u64,
}

/// Why an [`AlsoOpen`] run may have been closed after all: a record after
/// its opening record would not decode.
const CLOSE_UNDECODED: &str = "a record after its opening record could not be decoded and may be \
                               its closing record";

/// Why an [`AlsoOpen`] run may have been closed after all: it is known only
/// through a restated head, and the run records before the scan, where its
/// closing record may lie, could not be read.
const CLOSE_UNREAD: &str = "it is known only through a restated head, and the run records before \
                            the scan, where its closing record may lie, could not be read through \
                            the journal's index";

/// [`restart_scan`]'s answer once no damaged record is in doubt: `found` is
/// the run the readable records leave open, `before` the damaged records
/// strictly before its opening record, and `unread` why the run records
/// before the scan could not be read through the index, when they could not.
fn verdict(
    found: Option<Interrupted>,
    before: &[Damaged],
    unread: Option<String>,
    span: Span,
) -> Restart {
    let Span {
        records,
        scan_first,
        settled,
        ..
    } = span;
    match found {
        // THE INDEX COULD NOT BE READ AND THE RUN IS KNOWN ONLY THROUGH A
        // RESTATED HEAD: its opening record lies before the scan, and so may
        // the closing record that makes the restatement no new run. What the
        // scan read cannot tell (DB-B34, repair 3).
        Some(run) if run.ordinal < scan_first && unread.is_some() => not_known(format!(
            "the journal's newest {INTERRUPTED_SCAN} of {records} records restate a run opened \
             at record {} that they do not close, and the run records before them could not be \
             read through its index ({}), so whether that run was closed before them is not \
             known",
            run.ordinal,
            unread.unwrap_or_default()
        )),
        Some(mut run) => {
            decorate(&mut run, before, unread, span);
            logged(&run, before.len() as u64);
            Restart::Interrupted(run)
        }
        // THE INDEX COULD NOT BE READ: never a plain "clean" (DB-B34, repair
        // 3). It answered "clean" whenever the newest 800 records held any
        // opening or closing run record, dropping why it could not look
        // further back, on a journal whose older run was open -- the same
        // journal read "interrupted" with its index whole, and nothing was
        // logged for that start.
        None => match unread {
            Some(why) => not_known(format!(
                "the journal's newest {INTERRUPTED_SCAN} of {records} records {}, and the run \
                 records before them could not be read through its index ({why}), so a run \
                 opened before them and never closed would not be seen",
                if settled {
                    "leave no run they open unclosed"
                } else {
                    "hold no run record"
                }
            )),
            None => Restart::Clean,
        },
    }
}

/// What the scan read, set on the run the walk's reading leaves open and on
/// every other one it carries: how whole its failure count is, the damaged
/// records before it, whether a newer run may be open, what the index could
/// not tell and, for each other run, what may have closed it after all.
/// Shared by an interrupted answer and a guessed one's reading.
fn decorate(run: &mut Interrupted, before: &[Damaged], unread: Option<String>, span: Span) {
    let scan_first = span.scan_first;
    // The count is the run's whole count only when every record after its
    // opening record was read.
    run.failures_whole = run.failures_whole && run.ordinal >= scan_first;
    run.undecodable_before = before.len() as u64;
    run.undecodable = damaged_list(before);
    // A record before it that would not decode, and a closing record after
    // that one the walk could not pair by position: the close may belong to
    // the damaged record, and the run the walk closed with it may be open and
    // newer (the third check's `v62r3_p2`).
    run.newer_may_be_open = before
        .first()
        .zip(span.weak_close)
        .is_some_and(|((oldest, _), weak)| weak > *oldest);
    // Its opening record and every record after it were read, so it IS open;
    // what the index would have added is an older open run, and that it could
    // not is said.
    run.older_unread = unread;
    run.runs_unread = span.runs_unread;
    // EVERY OTHER RUN LEFT OPEN, with what may have closed it (AG-01, code-3):
    // a record after its opening record that would not decode, or a close
    // before the scan that could not be read.
    for other in &mut run.also_open {
        other.failures_whole = other.failures_whole && other.ordinal >= scan_first;
        other.doubt = if before.iter().any(|(at, _)| *at > other.ordinal) {
            Some(CLOSE_UNDECODED)
        } else if other.ordinal < scan_first && run.older_unread.is_some() {
            Some(CLOSE_UNREAD)
        } else {
            None
        };
    }
}

/// [`restart_scan`]'s answer when a live guess decides it (AG-01): NOT
/// KNOWN, carrying the walk's reading decorated as an interrupted answer's
/// run is, the runs no guess can move, and the counts a guessed close surely
/// left open; logged as an interrupted answer is for the runs that are sure,
/// besides the WARN line every NOT KNOWN answer writes.
fn guessed_answer(
    guessed: &[Guess],
    found: Option<Interrupted>,
    left_open: Vec<LeftOpen>,
    before: &[Damaged],
    unread: Option<&str>,
    span: Span,
) -> Restart {
    let why = guessed_words(guessed, found.as_ref(), unread);
    let named: std::collections::BTreeSet<u64> = guessed
        .iter()
        .flat_map(|guess| guess.names().iter().copied())
        .collect();
    let reading = found.map(|mut run| {
        decorate(&mut run, before, unread.map(str::to_owned), span);
        run
    });
    let mut sure = Vec::new();
    let mut doubted = Vec::new();
    let mut unnamed = 0;
    if let Some(run) = &reading {
        // The run read last is open unless a guess names it, or it is known
        // only through a restated head whose close may lie before an index
        // that could not be read; the records before it that would not
        // decode cannot be its close.
        let restated_unread = run.ordinal < span.scan_first && run.older_unread.is_some();
        if !named.contains(&run.ordinal) {
            let mut newest = AlsoOpen::from(run);
            if restated_unread {
                newest.doubt = Some(CLOSE_UNREAD);
                doubted.push(newest);
            } else {
                sure.push(newest);
            }
        }
        // EVERY OTHER RUN OF THE READING IS SAID: sure, or doubted with why
        // (third check of AG-01: a doubted run of a guessed answer was in
        // `reading` only, and its words never named it).
        for other in run
            .also_open
            .iter()
            .filter(|other| !named.contains(&other.ordinal))
        {
            if other.doubt.is_none() {
                sure.push(other.clone());
            } else {
                doubted.push(other.clone());
            }
        }
        unnamed = run
            .open_count
            .saturating_sub(1)
            .saturating_sub(run.also_open.len() as u64);
    }
    // A COUNT IS A FACT ONLY WHERE NOTHING READ COULD MOVE IT: no record that
    // would not decode after the oldest of its runs' opening records (it may
    // be one of their closes), and none of its runs known only through a
    // restated head whose close may lie before an index that could not be
    // read.
    let left_open = left_open
        .into_iter()
        .filter(|counted| {
            let oldest = counted.runs.iter().copied().min().unwrap_or(0);
            before.iter().all(|(at, _)| *at < oldest)
                && (unread.is_none() || oldest >= span.scan_first)
        })
        .take(OPEN_NAMED)
        .collect();
    let answer = Guessed {
        why,
        reading,
        guessed_runs: named.into_iter().take(OPEN_NAMED).collect(),
        sure,
        doubted,
        unnamed,
        left_open,
    };
    let words = answer.words();
    warned(&words);
    surely_logged(&answer);
    Restart::Guessed(Box::new(answer))
}

/// A guessed answer's ERROR lines (AG-01, second check's blocking 2): "a
/// pull run has no closing record" for each run open whichever way the
/// guesses go, as an interrupted answer writes it for each of its runs, and
/// "pull runs have no closing record, which of them is not known" for each
/// guessed close that surely left runs open. A guessed answer wrote one WARN
/// line and nothing else, so `/logs` named no run the records surely leave
/// open.
fn surely_logged(answer: &Guessed) {
    let open_count = answer.reading.as_ref().map_or(0, |run| run.open_count);
    let reasons = answer
        .sure
        .iter()
        .map(|run| {
            format!(
                "INTERRUPTED: {}, open whichever way the restart scan's guesses are read; which \
                 other runs are open is NOT KNOWN, and the restart answer says why",
                run.clause()
            )
        })
        .collect::<Vec<_>>();
    let counts = answer
        .left_open
        .iter()
        .map(|counted| {
            format!(
                "the closing record at record {} could end only one of the runs opened at \
                 records {}, so {} of them have no closing record; which is not known",
                counted.close,
                counted
                    .runs
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                counted.left_open
            )
        })
        .collect::<Vec<_>>();
    let sure = answer.sure.iter().zip(&reasons).map(|(run, reason)| {
        telemetry::Event::error("pull.run", "a pull run has no closing record")
            .with("ordinal", telemetry::Value::Uint(run.ordinal))
            .with("failures", telemetry::Value::Uint(run.failures))
            .with("source", telemetry::Value::Str(&run.source))
            .with("open_count", telemetry::Value::Uint(open_count))
            .with("guessed", telemetry::Value::Bool(true))
            .with("reason", telemetry::Value::Str(reason))
    });
    let counted = answer
        .left_open
        .iter()
        .zip(&counts)
        .map(|(counted, reason)| {
            telemetry::Event::error(
                "pull.run",
                "pull runs have no closing record, which of them is not known",
            )
            .with("close", telemetry::Value::Uint(counted.close))
            .with("runs", telemetry::Value::Uint(counted.runs.len() as u64))
            .with("left_open", telemetry::Value::Uint(counted.left_open))
            .with("source", telemetry::Value::Str(&counted.source))
            .with("reason", telemetry::Value::Str(reason))
        });
    emitted_all(sure.chain(counted));
}

/// Writes `events` to the installed sink: the ONE emit site of every ERROR
/// line the restart scan writes, interrupted and guessed alike, so
/// `emitted`'s count of sites is what it was.
fn emitted_all<'a>(events: impl Iterator<Item = telemetry::Event<'a>>) {
    for event in events {
        let _dropped_when_filtered = telemetry::emit(&event);
    }
}

/// The ERROR line "a pull run has no closing record", ONCE FOR EACH RUN LEFT
/// OPEN that the document names, each with its own ordinal, and one more for
/// the runs counted and not named (AG-01, code-3). It was written for the
/// newest run only, so `/logs` named one run of several cut short.
///
/// ONE EMIT SITE, as before: the lines are built first and written by the
/// same call, so `emitted`'s count of sites is unchanged and the site stays
/// the one `server::tests::a_run_cut_short_by_a_restart_is_logged_when_it_is_found`
/// reads back. The lines it now writes beside the first are read back by
/// `restart_tests::three_runs_left_open_are_all_named` (one per run named)
/// and `restart_tests::the_open_list_stops_at_the_scan_bound_and_says_so`
/// (the line for the runs counted and not named).
fn logged(run: &Interrupted, undecodable_before: u64) {
    let words = run.words();
    let reasons = run
        .also_open
        .iter()
        .map(|other| {
            format!(
                "INTERRUPTED: {}, one of {} runs read with no closing record; the one read last \
                 is record {}, and its line says what is re-run and how",
                other.clause(),
                run.open_count,
                run.ordinal
            )
        })
        .collect::<Vec<_>>();
    let unnamed = run
        .open_count
        .saturating_sub(1)
        .saturating_sub(run.also_open.len() as u64);
    let newest = telemetry::Event::error("pull.run", "a pull run has no closing record")
        .with("ordinal", telemetry::Value::Uint(run.ordinal))
        .with("failures", telemetry::Value::Uint(run.failures))
        .with("source", telemetry::Value::Str(&run.source))
        .with(
            "undecodable_before",
            telemetry::Value::Uint(undecodable_before),
        )
        .with("undecodable", telemetry::Value::Str(&run.undecodable))
        .with(
            "newer_may_be_open",
            telemetry::Value::Bool(run.newer_may_be_open),
        )
        .with(
            "older_unread",
            telemetry::Value::Str(run.older_unread.as_deref().unwrap_or("")),
        )
        .with("open_count", telemetry::Value::Uint(run.open_count))
        .with("reason", telemetry::Value::Str(&words));
    let others = run.also_open.iter().zip(&reasons).map(|(other, reason)| {
        telemetry::Event::error("pull.run", "a pull run has no closing record")
            .with("ordinal", telemetry::Value::Uint(other.ordinal))
            .with("failures", telemetry::Value::Uint(other.failures))
            .with("source", telemetry::Value::Str(&other.source))
            .with("open_count", telemetry::Value::Uint(run.open_count))
            .with("reason", telemetry::Value::Str(reason))
    });
    let counted = (unnamed > 0).then(|| {
        telemetry::Event::error("pull.run", "pull runs have no closing record, not named")
            .with("unnamed", telemetry::Value::Uint(unnamed))
            .with("open_count", telemetry::Value::Uint(run.open_count))
    });
    emitted_all(core::iter::once(newest).chain(others).chain(counted));
}

/// A record the restart scan was handed, or was told of by the journal's
/// index, and could not decode: its ordinal, and why in the decoder's words.
/// A record the index files is one fixed sentence, borrowed, so a journal
/// with many filed faults costs 32 bytes for each in the walk's span and no
/// allocation (repair 3; the span's count is bounded by the journal, named
/// in `docs/06-limits.md`).
type Damaged = (u64, std::borrow::Cow<'static, str>);

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
    warned(&why);
    Restart::NotKnown(why)
}

/// The WARN line every NOT KNOWN answer writes, with its reason: the one
/// emit site of it, for a bare answer and a guessed one alike.
fn warned(why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.run",
            "whether a pull run was left open is not known",
        )
        .with("reason", telemetry::Value::Str(why)),
    );
}

/// What [`older_runs`] read before the scan.
struct Older {
    /// The records that decoded, oldest first.
    read: Vec<(u64, crate::audit::Record)>,
    /// The records that did not, or that the index files as undecodable and
    /// were not read again.
    damaged: Vec<Damaged>,
    /// Run records the index names below the scan that were NOT read: those
    /// behind the [`INTERRUPTED_RUNS`] newest (AG-01, code-3).
    runs_unread: u64,
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
        // `build_index` builds an index that is absent and CATCHES UP one
        // that is behind; it does not rewrite entries it already covers, so
        // "rebuilt" overstated what was done about an unreadable entry. The
        // same reason twice is said once.
        .map_err(|why| {
            if why == first {
                format!("{first}; it was built or caught up here, and that still holds")
            } else {
                format!("{first}; it was built or caught up here, and then: {why}")
            }
        })
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
        runs_unread: below.saturating_sub(INTERRUPTED_RUNS) as u64,
        damaged: as_filed
            .iter()
            .map(|at| {
                (
                    *at,
                    std::borrow::Cow::Borrowed(
                        "the journal's index files it as a record that would not decode",
                    ),
                )
            })
            .collect(),
    };
    for (ordinal, decoded) in journal.records_at(&ordinals)? {
        match decoded {
            Ok(record) => out.read.push((ordinal, record)),
            Err(fault) => out.damaged.push((ordinal, fault.to_string().into())),
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
            // THE LEGS NO ANSWER COVERS (AG-01, code-2): by wire name, read
            // from the vendor table, so a page can say which feeds' runs the
            // answer above cannot speak for.
            out.push_str(",\"unjournalledFeeds\":[");
            for (at, feed) in unjournalled_feeds().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                out.push_str(&quote_for_json(feed.wire()));
            }
            out.push(']');
            // A GUESSED ANSWER'S KNOWN HALF (AG-01, second check's blocking
            // 2): the walk's reading, the runs no guess can move and the
            // counts a guessed close surely left open. Only on that answer, so
            // every other answer's document is what it was.
            if let Restart::Guessed(guessed) = state {
                guessed.json(&mut out);
            }
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
/// own feeds write to: what the page shows as the store's count
/// (`rowsAtStart`, `rowsNow`). It is NOT the run's progress any more (repair
/// 4): another writer of the same vendor moves it too, so the growth test
/// and the bars figure read [`Progress::bars_written`], the run's own
/// receipts.
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

/// The receipt facts a leg states its written bars under: a spot landing's
/// (`server::landed_answer`, `bars_committed`) and an F&O walk's
/// (`Rolled::record_landing` sums `bars_committed` into it). Read in this
/// order, the first row wins.
const BARS_WRITTEN_FACTS: [&str; 2] = ["Bars written to the file", "Bars stored"];

/// The bars a leg's receipt says the leg wrote, or `None` when it states no
/// such count, or one that is not a whole number (DB-B33, repair 4).
fn bars_written_in(html: &str) -> Option<u64> {
    let said = BARS_WRITTEN_FACTS
        .iter()
        .find_map(|label| receipt_fact(html, label))?;
    said.split_whitespace().next()?.parse().ok()
}

/// The run's written-bars figure after one leg: `total` plus what the leg's
/// receipt stated. A STORED receipt that states no count makes the figure
/// unknown for the rest of the run, as an overflowing sum does: a figure
/// that silently skipped a landing would read as a run that wrote less. Any
/// other receipt that states none (a refusal before the first landing) wrote
/// nothing this run can count, and leaves it as it was.
fn add_written(total: Option<u64>, wrote: Option<u64>, stored: bool) -> Option<u64> {
    match wrote {
        Some(bars) => total?.checked_add(bars),
        None if stored => None,
        None => total,
    }
}

/// The written-bars figure of run `run`, or `None` when it is unknown or
/// the slot holds another run.
fn written_of(site: &Site, run: u64) -> Option<u64> {
    site.run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .filter(|progress| progress.generation == run)
        .and_then(|progress| progress.bars_written)
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
        let outcome = leg_outcome(status, &html);
        let wrote = bars_written_in(&html);
        with_progress(&site, run, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.legs_done = feed.legs_done.saturating_add(1);
            }
            progress.bars_written =
                add_written(progress.bars_written, wrote, outcome == LegOutcome::Stored);
        });
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
/// Idle termination is an observation about the bars the run's own legs
/// wrote (DB-B33), never full coverage.
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
    // THE STORE'S COUNT FOR THE RUN'S OWN VENDORS, shown as what the store
    // holds (`rowsAtStart`, `rowsNow`). Another writer of the same vendor
    // moves it too, so it is not the run's progress: that is the bars its
    // own legs' receipts say they wrote, `bars_written` (DB-B33, repair 4).
    let own = own_vendors(&groups);
    let started_rows = rows_of(&site, &own);
    with_progress(&site, run, |progress| {
        progress.rows_at_start = started_rows;
        progress.rows_now = started_rows;
        progress.bars_written = Some(0);
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
        // GROWTH IS WHAT THIS RUN'S OWN LEGS WROTE (DB-B33, repair 4): the
        // census of its vendor is moved by any other writer of that vendor.
        let before = written_of(&site, run);
        run_pass(&site, run, &groups, &mut outcomes, &checkpoints, &request).await;
        passes = passes.saturating_add(1);
        let after = written_of(&site, run);
        let census = rows_of(&site, &own);
        with_progress(&site, run, |progress| {
            progress.passes = passes;
            progress.rows_now = census;
        });
        if stopping(&site, run)
            || outcomes
                .iter()
                .all(|outcome| *outcome == PassOutcome::Halted)
        {
            break;
        }
        // Growth is proven only by two known counts; an unknown one (CE-74,
        // or a STORED receipt that stated no count) proves nothing, so the
        // pass is judged as if nothing landed.
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
        progress.finished = Some(run_summary(progress, &outcomes, progress.bars_written));
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
            "INCOMPLETE after {} pass(es); {landed_said} bar(s) written by this run's own legs. \
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

/// The bars-written figure in words: the number, or `an unknown number of`
/// when the sum did not fit a `u64` (CE-74, D-1776) or a STORED receipt
/// stated no count (DB-B33, repair 4).
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
            "Stopped after {passes} pass(es); {landed} bar(s) written by this run's own legs \
             before you pressed stop.{note} Full basket coverage has not been verified."
        );
    }
    if passes >= MAX_PASSES {
        return format!(
            "Reached the {MAX_PASSES}-pass ceiling with {landed} bar(s) written by this run's own legs.{note} \
             This is a runaway stop and NOT a verdict on the window — press Pull \
             again to continue. Full basket coverage has not been verified."
        );
    }
    format!(
        "Idle retries stopped after {passes} pass(es); {landed} bar(s) written by this run's own legs.{note} \
         The last {CLEAN_EMPTY_PASSES} passes reported no retryable failures and wrote no bar. \
         Empty responses or already-present bars can explain this. \
         Full basket coverage has not been verified."
    )
}

#[cfg(test)]
#[path = "pullrun_restart_tests.rs"]
mod restart_tests;

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
            bars_written: Some(375),
        };
        let doc = progress.json();
        for key in [
            "\"barsWritten\":375",
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

    /// A synthetic receipt that, as a landing's receipt does, states the
    /// bars its leg wrote: none.
    fn receipt(status: u16, verdict: &str) -> (axum::http::StatusCode, String) {
        receipt_wrote(status, verdict, Some(0))
    }

    /// A synthetic receipt stating `wrote` bars under the spot landing's
    /// label, or no count at all for `None`.
    fn receipt_wrote(
        status: u16,
        verdict: &str,
        wrote: Option<u64>,
    ) -> (axum::http::StatusCode, String) {
        let facts: Vec<(&'static str, String)> = wrote
            .map(|bars| vec![(BARS_WRITTEN_FACTS[0], bars.to_string())])
            .unwrap_or_default();
        (
            axum::http::StatusCode::from_u16(status).expect("HTTP status"),
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict,
                reason: "synthetic coordinator receipt",
                good: verdict == "STORED",
                facts: &facts,
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
            summary.contains("Idle retries stopped")
                && summary.contains("; 0 bar(s) written by this run's own legs"),
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

    /// **ANOTHER WRITER OF THE SAME VENDOR IS NOT THIS RUN'S PROGRESS**
    /// (DB-B33, repair 4; the third check's follow-up 10). The Zerodha
    /// census grows on every request, as a `cli` pull or a second run of
    /// the same vendor would make it, while this run's receipts say they
    /// wrote nothing. Counted on the vendor's census the run read that as
    /// its own progress on every pass (never idle, no retry wait, another
    /// writer's rows credited to it); counted from its own receipts it stops
    /// after the three idle passes having written nothing, and the census is
    /// still shown as what the store holds.
    #[tokio::test]
    async fn another_writer_of_the_same_vendor_neither_keeps_the_run_alive_nor_counts_as_its_bars()
    {
        let grown = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let grow = std::sync::Arc::clone(&grown);
        let progress = simulated(
            "same-vendor-growth",
            vec![leg("zerodha", "1day")],
            move |site, _leg| {
                let rows = grow.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                write_census(&site, brutex_core::vendor::Vendor::Zerodha, rows);
                std::future::ready(receipt(200, "STORED"))
            },
        )
        .await;
        assert_eq!(progress.passes, CLEAN_EMPTY_PASSES, "{progress:?}");
        assert_eq!(progress.stop, Some(Stop::Idle));
        assert_eq!(progress.bars_written, Some(0));
        assert_eq!(
            (progress.rows_at_start, progress.rows_now),
            (Some(0), Some(u64::from(CLEAN_EMPTY_PASSES))),
            "the census is shown as the store's count"
        );
        assert!(progress.json().contains("\"barsWritten\":0"));
        let summary = progress.finished.expect("summary");
        assert!(
            summary.contains("; 0 bar(s) written by this run's own legs"),
            "{summary}"
        );
    }

    /// The run's own receipts prove its growth: a leg that wrote bars on the
    /// first pass keeps the run going, the three idle passes follow, and the
    /// figure is the receipts' sum. A STORED receipt that states no count
    /// makes the figure unknown, never a short number, and proves no growth.
    #[tokio::test]
    async fn the_runs_own_receipts_prove_its_growth_and_an_unstated_count_is_unknown() {
        let asked = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let ask = std::sync::Arc::clone(&asked);
        let progress = simulated(
            "own-receipt-growth",
            vec![leg("zerodha", "1day")],
            move |_site, _leg| {
                let nth = ask.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::future::ready(receipt_wrote(
                    200,
                    "STORED",
                    Some(if nth == 0 { 375 } else { 0 }),
                ))
            },
        )
        .await;
        assert_eq!(progress.passes, CLEAN_EMPTY_PASSES + 1, "{progress:?}");
        assert_eq!(progress.bars_written, Some(375));
        assert!(
            progress
                .finished
                .as_deref()
                .is_some_and(|said| said.contains("; 375 bar(s) written")),
            "{:?}",
            progress.finished
        );

        let unstated = simulated(
            "own-receipt-unstated",
            vec![leg("zerodha", "1day")],
            |_site, _leg| std::future::ready(receipt_wrote(200, "STORED", None)),
        )
        .await;
        assert_eq!(unstated.passes, CLEAN_EMPTY_PASSES);
        assert_eq!(unstated.bars_written, None);
        assert!(
            unstated
                .finished
                .as_deref()
                .is_some_and(|said| said.contains("an unknown number of bar(s)")),
            "{:?}",
            unstated.finished
        );
        assert!(unstated.json().contains("\"barsWritten\":null"));
    }

    /// The written-bars figure, step by step: each label, a count that is
    /// not a number, an empty refusal, a STORED receipt with no count, and a
    /// sum that does not fit.
    #[test]
    fn the_written_bars_figure_reads_each_receipt_label_and_never_guesses() {
        let page = |label: &'static str, said: &str| {
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict: "STORED",
                reason: "synthetic",
                good: true,
                facts: &[(label, said.to_owned())],
                footnote: "No vendor was contacted.",
            })
        };
        assert_eq!(
            bars_written_in(&page("Bars written to the file", "375")),
            Some(375)
        );
        assert_eq!(bars_written_in(&page("Bars stored", "12")), Some(12));
        assert_eq!(
            bars_written_in(&page("Bars written to the file", "many")),
            None
        );
        assert_eq!(
            bars_written_in(&page("Bars written to the file", "-1")),
            None
        );
        assert_eq!(bars_written_in(&page("Rows read", "9")), None);
        assert_eq!(add_written(Some(5), Some(7), true), Some(12));
        assert_eq!(
            add_written(Some(5), None, false),
            Some(5),
            "a refusal wrote nothing"
        );
        assert_eq!(
            add_written(Some(5), None, true),
            None,
            "a STORED with no count"
        );
        assert_eq!(
            add_written(None, Some(7), true),
            None,
            "unknown stays unknown"
        );
        assert_eq!(
            add_written(Some(u64::MAX), Some(1), true),
            None,
            "no overflow"
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

    /// The ask [`opened_at`] names: one pull, made by every run of these
    /// tests that does not say otherwise.
    const ASK: u64 = 0x00a5_c0de_0000_0001;

    fn opened_at(secs: u64, source: &str, from: u32, to: u32) -> crate::audit::Record {
        opened_asking(secs, source, from, to, ASK)
    }

    /// An opening record naming `ask`.
    fn opened_asking(
        secs: u64,
        source: &str,
        from: u32,
        to: u32,
        ask: u64,
    ) -> crate::audit::Record {
        let day = |at| pull::session::Day::from_days(at).expect("a day");
        crate::audit::Record::opened(
            crate::audit::Scope::Spot,
            std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs),
            source,
            pull::session::Window::new(day(from), day(to)).expect("a window"),
            7,
            ask,
        )
    }

    /// An opening record as every build before D-4950 wrote it: no ask.
    fn opened_unasked(secs: u64, source: &str, from: u32, to: u32) -> crate::audit::Record {
        let mut head = opened_at(secs, source, from, to);
        head.note = "opened for 7 instruments; failures follow as they land".to_owned();
        head.note_bytes = u16::try_from(head.note.len()).expect("a short note");
        head
    }

    /// A record of `scope` and `outcome` for the window, as a press writes it.
    fn record_of(
        scope: crate::audit::Scope,
        outcome: crate::audit::Outcome,
        secs: u64,
        source: &str,
        from: u32,
        to: u32,
    ) -> crate::audit::Record {
        let mut record = closed_at(secs, source, from, to);
        record.scope = scope;
        record.outcome = outcome;
        record
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
        // The same, with the opening record before the records read. The
        // close may be the run's own (as above) or ANOTHER run's of the same
        // scope, window and source, and neither record names an ask, so the
        // two shapes are one: NOT KNOWN, never "clean" (AG-01, code-7). It
        // read None, and a live run cut short behind another run's close read
        // clean. The walk's own reading keeps the run open (D-4976).
        let unseen = numbered(
            vec![
                closed_at(120, "kite", 20_000, 20_010),
                again.clone(),
                failure_at(121),
            ],
            900,
        );
        assert!(
            matches!(restart_in(&unseen), Restart::Guessed(_)),
            "{:?}",
            restart_in(&unseen)
        );
        assert_eq!(restart_in(&unseen).word(), "not_known");
        assert_eq!(interrupted_in(&unseen).map(|run| run.ordinal), Some(0));
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

    /// **A RUN THAT REACHED NO INSTRUMENT CLOSES UNDER ITS OWN SOURCE, AND A
    /// RECORD UNDER A TARGET'S LABEL CLOSES NOTHING** (DB-B34, repair 6).
    #[test]
    fn a_close_under_its_own_source_ends_its_run_and_a_target_label_does_not() {
        // A run that reached nobody is closed NOT STARTED under its OWN
        // source, and that closes it (DB-B34, repair 6) ...
        let reached_nobody = record_of(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::NotStarted,
            118,
            "kite",
            20_000,
            20_010,
        );
        assert_eq!(
            interrupted_in(&numbered(
                vec![opened_at(100, "kite", 20_000, 20_010), reached_nobody],
                0
            )),
            None
        );
        // ... and a NOT STARTED record under a source no open run carries
        // (the TARGET's label, as a press refused before its run opened wrote
        // it, and as the arm above wrote its close before this repair)
        // closes NOTHING: the window alone paired them, and a refusal that
        // asked the vendor for nothing read the run cut short as closed.
        let target_label = record_of(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::NotStarted,
            118,
            "fno",
            20_000,
            20_010,
        );
        assert_eq!(
            interrupted_in(&numbered(
                vec![opened_at(100, "kite", 20_000, 20_010), target_label],
                0
            ))
            .map(|run| run.ordinal),
            Some(0)
        );
    }

    /// **A PRESS REFUSED BEFORE ITS RUN OPENED CLOSES NO RUN, OF EITHER
    /// SCOPE** (DB-B34, repair 6; the fourth check's blocking issue 1 and its
    /// blobs S1..S4 and T1, T2). The pull order's "the 1day pass comes
    /// first", a mapping refusal or a refused broker wrote ONE record, NOT
    /// STARTED under the target's label with the window it asked for, and no
    /// opening record. The walk read it as the close of the newest open run
    /// of that window, so the run cut short read as closed and the next start
    /// answered "clean". The operator's own journal holds four such records
    /// (ordinals 32, 33, 223935, 4217091).
    #[test]
    fn a_refusal_that_opened_no_run_closes_no_run_of_either_scope() {
        use crate::audit::{Outcome, Scope};
        // [Opened A][failures][orphan NOT STARTED, same window]: A is open.
        let mut journal = vec![opened_at(100, "endpoint A", 17_532, 20_726)];
        journal.extend((0..5).map(|at| failure_at(110 + at)));
        journal.push(record_of(
            Scope::Spot,
            Outcome::NotStarted,
            200,
            "zerodha_nse_checked NOT vendor-ISIN; NSE file same day; Reference indices",
            17_532,
            20_726,
        ));
        assert_eq!(
            interrupted_in(&numbered(journal.clone(), 0)).map(|run| (run.ordinal, run.failures)),
            Some((0, 5)),
            "a refusal that opened no run asked the vendor for nothing"
        );
        // The fourth check's S1 -> S4: another run of another window opened
        // and closed between, failures after it, then the orphan.
        let mut s4 = vec![opened_at(100, "endpoint A", 17_532, 20_726)];
        s4.extend((0..5).map(|at| failure_at(110 + at)));
        s4.push(opened_at(150, "endpoint A", 17_897, 17_927));
        s4.push(closed_at(160, "endpoint A", 17_897, 17_927));
        s4.extend((0..3).map(|at| failure_at(170 + at)));
        s4.push(journal.last().cloned().expect("the orphan"));
        assert_eq!(
            interrupted_in(&numbered(s4, 4_217_222)).map(|run| run.ordinal),
            Some(4_217_222)
        );
        // The same press written as this build writes it, REFUSED: closes
        // nothing either, whatever its source.
        let refused = numbered(
            vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                record_of(Scope::Spot, Outcome::Refused, 120, "kite", 20_000, 20_010),
            ],
            0,
        );
        assert_eq!(interrupted_in(&refused).map(|run| run.ordinal), Some(0));

        // CROSS-SCOPE (T1 -> T2): an expired-F&O walk cut short is not closed
        // by a SPOT record of the same window, even one naming the same
        // source; its own F&O close does close it.
        let mut walk = opened_at(100, "NIFTY", 18_262, 18_292);
        walk.scope = Scope::Fno;
        for spot in [
            record_of(
                Scope::Spot,
                Outcome::NotStarted,
                120,
                "NIFTY",
                18_262,
                18_292,
            ),
            record_of(Scope::Spot, Outcome::Stored, 120, "NIFTY", 18_262, 18_292),
        ] {
            let found = interrupted_in(&numbered(vec![walk.clone(), spot], 4_217_222))
                .expect("the walk is open");
            assert_eq!(
                (found.ordinal, found.scope, found.source.as_str()),
                (4_217_222, Scope::Fno, "NIFTY")
            );
        }
        let own = record_of(
            Scope::Fno,
            Outcome::NotStarted,
            120,
            "NIFTY",
            18_262,
            18_292,
        );
        assert_eq!(interrupted_in(&numbered(vec![walk, own], 0)), None);
    }

    /// **A CLOSE PAIRS WITH THE RUN IT DIRECTLY FOLLOWS** (DB-B34, repair 6;
    /// D-4950). Two runs of one window and source in flight at once (a
    /// server run and a `cli` pull): the close of the OLDER, written through
    /// its own thread, directly follows a record of it (here its restated
    /// head), and closes it, not the newer one. A close that follows no
    /// record of a run of its own (written before D-4950) still takes the
    /// newest open run of its scope, window and source.
    #[test]
    fn a_close_pairs_with_the_run_it_directly_follows() {
        let mut again = opened_at(100, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 0".to_owned();
        let beside = numbered(
            vec![
                opened_asking(100, "kite", 20_000, 20_010, ASK),
                opened_asking(110, "kite", 20_000, 20_010, ASK ^ 2),
                again,
                closed_at(130, "kite", 20_000, 20_010),
            ],
            0,
        );
        assert_eq!(
            interrupted_in(&beside).map(|run| (run.ordinal, run.ask)),
            Some((1, Some(ASK ^ 2))),
            "the older run closed; the newer one is open"
        );
        // Behind its own member failure, too.
        let behind_failure = numbered(
            vec![
                opened_asking(100, "kite", 20_000, 20_010, ASK),
                opened_asking(110, "kite", 20_000, 20_010, ASK ^ 2),
                {
                    let mut again = opened_at(100, "kite", 20_000, 20_010);
                    again.note = "continues the run opened at record 0".to_owned();
                    again
                },
                failure_at(125),
                closed_at(130, "kite", 20_000, 20_010),
            ],
            0,
        );
        assert_eq!(
            interrupted_in(&behind_failure).map(|run| run.ordinal),
            Some(1)
        );
        // A close with a gap before it (another writer's record lies between,
        // or one that would not decode) is not by position: the newest open
        // run of its scope, window and source.
        let gap = vec![
            (0, opened_asking(100, "kite", 20_000, 20_010, ASK)),
            (1, opened_asking(110, "kite", 20_000, 20_010, ASK ^ 2)),
            (5, closed_at(130, "kite", 20_000, 20_010)),
        ];
        assert_eq!(interrupted_in(&gap).map(|run| run.ordinal), Some(0));
        // A run whose opening record was refused and is written LATE, in one
        // write with its close (`audit::Journal::close_unopened`), closes
        // itself and leaves a cut-short run of another ask over the same
        // window open.
        let mut late = opened_asking(150, "kite", 20_000, 20_010, ASK ^ 4);
        late.note.push_str("; late");
        assert_eq!(crate::audit::ask_of(&late.note), Some(ASK ^ 4));
        let mut read = vec![opened_at(100, "kite", 20_000, 20_010)];
        read.extend((0..3).map(|at| failure_at(110 + at)));
        read.push(late);
        read.push(closed_at(160, "kite", 20_000, 20_010));
        assert_eq!(
            interrupted_in(&numbered(read, 0)).map(|run| run.ordinal),
            Some(0)
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
            assert!(why.contains("built here"), "{why}");
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
    /// opening record is NOT KNOWN, and so is damage AT it (repair 3: a
    /// restated opening record whose own first record is the damaged one
    /// does not name the run, since its close may have paired by window).
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
        // REPAIR 3: damage AT the run's own opening record is NOT KNOWN. It
        // was reported INTERRUPTED; but a run known only through its
        // restatement may have been closed under its target's name, a
        // pairing by window alone that needs the opening record.
        let found = restart_scan(&journal);
        assert!(
            not_known_of(&found).contains("record 0 (not an audit record)"),
            "{found:?}"
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

    /// **AN INDEX THAT CANNOT BE READ NEVER LEAVES THE ANSWER "CLEAN"**
    /// (DB-B34, repair 3; the third check's blocking issue). The journal
    /// `[opened A][850 failures][opened B][closed B][10 failures]` has A
    /// open behind the scan. With its index whole it reads INTERRUPTED at
    /// record 0. With the index's `runs` entries unreadable, and with the
    /// index gone and `audit/` taking no write, it read CLEAN with no log
    /// line, because the scan held B's opening and closing records. Each is
    /// NOT KNOWN now, naming the index's reason.
    #[test]
    fn an_index_that_cannot_be_read_never_leaves_the_answer_clean() {
        let shaped = |closed_b: bool| {
            let mut records = vec![opened_at(100, "kite", 20_000, 20_010)];
            records.extend((0..850).map(|at| failure_at(200 + at)));
            records.push(opened_at(2_000, "kite", 20_020, 20_030));
            if closed_b {
                records.push(closed_at(2_010, "kite", 20_020, 20_030));
            }
            records.extend((0..10).map(|at| failure_at(2_100 + at)));
            records
        };
        // (1) Whole: A is found through the index.
        let journal = journal_of("pullrun-r3-index-whole", &shaped(true));
        assert_eq!(
            restart_scan(&journal).interrupted().map(|run| run.ordinal),
            Some(0),
            "with its index whole the journal says A is open"
        );
        // (2) Every `runs` entry unreadable.
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entries are overwritten");
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(
            why.contains("of the index's entries cannot be read"),
            "{why}"
        );
        assert!(why.contains("leave no run they open unclosed"), "{why}");
        assert!(why.contains("would not be seen"), "{why}");
        assert!(found.words().starts_with("NOT KNOWN"));
        // A catch-up does not rewrite entries it already covers, so the
        // reason says what was done and does not claim a rebuild.
        assert!(why.contains("built or caught up here"), "{why}");
        assert!(!why.contains("rebuilt"), "{why}");
        // The same reason before and after the catch-up is said once.
        assert!(why.contains("and that still holds"), "{why}");
        assert_eq!(
            why.matches("of the index's entries cannot be read").count(),
            1,
            "{why}"
        );

        // (3) No index, and `audit/` takes no write: it cannot be built.
        let journal = journal_of("pullrun-r3-index-readonly", &shaped(true));
        std::fs::rename(
            journal.index_dir(),
            journal.index_dir().with_extension("old"),
        )
        .expect("the index is moved aside");
        let audit = journal.index_dir().parent().expect("audit/").to_path_buf();
        set_mode(&audit, 0o555);
        let found = restart_scan_waiting(&journal, 1);
        set_mode(&audit, 0o755);
        let why = not_known_of(&found);
        assert!(why.contains("could not be built here"), "{why}");
        assert!(why.contains("would not be seen"), "{why}");

        // (4) With B left open the scan itself reads an open run whose
        // opening record and every record after it it read: B is reported,
        // and the document says the older run records were not read.
        let journal = journal_of("pullrun-r3-index-open-b", &shaped(false));
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entries are overwritten");
        let found = restart_scan(&journal);
        let run = found.interrupted().expect("B is open");
        assert_eq!(
            (run.ordinal, run.failures, run.failures_whole),
            (851, 10, true)
        );
        let unread = run.older_unread.clone().expect("the reason is kept");
        assert!(unread.contains("cannot be read"), "{unread}");
        assert!(
            run.words()
                .contains("an older run left open would not have been seen")
        );
        let doc: serde_json::Value =
            serde_json::from_str(&status_json(&Progress::default(), None, Some(&found)))
                .expect("valid JSON");
        assert_eq!(doc["interrupted"]["olderUnread"], unread.as_str());
        assert_eq!(doc["interrupted"]["undecodableBefore"], 0);

        // (5) A run known only through a restated head inside the scan,
        // with its opening record behind it and the index unreadable: its
        // close may lie behind the scan too, so NOT KNOWN.
        let mut again = opened_at(100, "kite", 20_000, 20_010);
        again.note = "continues the run opened at record 0".to_owned();
        let mut records = vec![opened_at(100, "kite", 20_000, 20_010)];
        records.extend((0..850).map(|at| failure_at(200 + at)));
        records.push(again);
        records.extend((0..5).map(|at| failure_at(2_100 + at)));
        let journal = journal_of("pullrun-r3-index-restated", &records);
        assert_eq!(
            restart_scan(&journal).interrupted().map(|run| run.ordinal),
            Some(0)
        );
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entries are overwritten");
        let found = restart_scan(&journal);
        let why = not_known_of(&found);
        assert!(why.contains("restate a run opened at record 0"), "{why}");
    }

    /// Sets `dir`'s permission bits.
    fn set_mode(dir: &std::path::Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }

    /// **A LATER RUN OF THE SAME WINDOW AND SOURCE THAT CLOSES SUPERSEDES
    /// AN INTERRUPTED ONE** (DB-B34, repair 3; the third check's follow-up
    /// 2). Nothing closed an interrupted run's opening record, so after the
    /// operator pulled the same window again and that run closed, every
    /// later start still answered INTERRUPTED, logged an ERROR and asked for
    /// the pull just made. A close of another source, of another window, or
    /// paired by its window alone supersedes nothing.
    #[test]
    fn a_later_run_of_the_same_window_and_source_that_closes_supersedes_an_open_one() {
        let interrupted_then = |later: Vec<crate::audit::Record>| {
            let mut read = vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                failure_at(111),
            ];
            read.extend(later);
            interrupted_in(&numbered(read, 0)).map(|run| run.ordinal)
        };
        assert_eq!(interrupted_then(Vec::new()), Some(0), "cut short");
        assert_eq!(
            interrupted_then(vec![
                opened_at(500, "kite", 20_000, 20_010),
                failure_at(510),
                closed_at(520, "kite", 20_000, 20_010),
            ]),
            None,
            "the same pull made again and closed"
        );
        assert_eq!(
            interrupted_then(vec![
                opened_at(500, "probe", 20_000, 20_010),
                closed_at(520, "probe", 20_000, 20_010),
            ]),
            Some(0),
            "another source's run says nothing of this one"
        );
        assert_eq!(
            interrupted_then(vec![
                opened_at(500, "kite", 20_000, 20_020),
                closed_at(520, "kite", 20_000, 20_020),
            ]),
            Some(0),
            "another window's run says nothing of this one"
        );
        // A close under a source no open run carries (a target's label)
        // closes NOTHING (repair 6): it was paired by its window alone with
        // the newest run of that window. Both runs stay open; the newest is
        // reported.
        assert_eq!(
            interrupted_then(vec![
                opened_at(500, "probe", 20_000, 20_010),
                closed_at(520, "target", 20_000, 20_010),
            ]),
            Some(3)
        );
        // A newer run of the same window and source that is itself still
        // open is the one reported, and nothing is superseded by it.
        assert_eq!(
            interrupted_then(vec![opened_at(500, "kite", 20_000, 20_010)]),
            Some(3)
        );
        // A later run of the same window and source that closed NOT STARTED
        // asked the vendor for nothing: it closes itself and supersedes
        // nothing (repair 5; it read the older run as done).
        let mut not_started = closed_at(520, "kite", 20_000, 20_010);
        not_started.outcome = crate::audit::Outcome::NotStarted;
        assert_eq!(
            interrupted_then(vec![opened_at(500, "kite", 20_000, 20_010), not_started]),
            Some(0),
            "a press that never started is no re-pull"
        );
        // The other closing outcomes did ask: an empty answer and a stored one
        // supersede as a failed one does.
        for outcome in [crate::audit::Outcome::Empty, crate::audit::Outcome::Stored] {
            let mut close = closed_at(520, "kite", 20_000, 20_010);
            close.outcome = outcome;
            assert_eq!(
                interrupted_then(vec![opened_at(500, "kite", 20_000, 20_010), close]),
                None,
                "{outcome:?}"
            );
        }

        // The same through a journal longer than the scan: the interrupted
        // run's opening record is read through the index.
        let mut records = vec![opened_at(100, "kite", 20_000, 20_010)];
        records.extend((0..INTERRUPTED_SCAN + 30).map(|at| failure_at(200 + at)));
        records.push(opened_at(9_000, "kite", 20_000, 20_010));
        records.push(closed_at(9_010, "kite", 20_000, 20_010));
        let journal = journal_of("pullrun-r3-superseded", &records);
        assert_eq!(restart_scan(&journal), Restart::Clean);
        // The clean answer says what counts as the same pull made again, so
        // "every run record it read is closed" is not read as "no run was ever
        // cut short", and no longer claims a re-pull from the window and
        // source alone (repair 6).
        let words = Restart::Clean.words();
        assert!(
            words.contains("whose opening record names the same ask")
                && words.contains("a refusal that opened no run closes nothing"),
            "{words}"
        );
        assert!(
            !words.contains("asked the vendor for the same days again"),
            "{words}"
        );
    }

    /// **ANOTHER ASK OVER THE SAME WINDOW AND SOURCE SUPERSEDES NOTHING, AND
    /// A RUN WITH NO ASK IS NEVER SUPERSEDED** (DB-B34, repair 6; the fourth
    /// check's blocking issue 2 and its blobs U1..U3).
    #[test]
    fn another_ask_over_the_same_window_and_source_supersedes_nothing() {
        let interrupted_then = |later: Vec<crate::audit::Record>| {
            let mut read = vec![
                opened_at(100, "kite", 20_000, 20_010),
                failure_at(110),
                failure_at(111),
            ];
            read.extend(later);
            interrupted_in(&numbered(read, 0)).map(|run| run.ordinal)
        };
        // THE SAME WINDOW AND SOURCE, ANOTHER ASK (repair 6; the fourth
        // check's probe `v62r4_p2`): every Zerodha spot run of a window
        // carries one source, so a later 1day pull of the indices closing
        // over the window of a cut-short 1min pull of the stocks superseded
        // it. The ask on the two opening records differs: nothing is
        // superseded, whatever the close (STORED, EMPTY or FAILED).
        for outcome in [
            crate::audit::Outcome::Stored,
            crate::audit::Outcome::Empty,
            crate::audit::Outcome::Failed,
        ] {
            let mut close = closed_at(520, "kite", 20_000, 20_010);
            close.outcome = outcome;
            assert_eq!(
                interrupted_then(vec![
                    opened_asking(500, "kite", 20_000, 20_010, ASK ^ 1),
                    close,
                ]),
                Some(0),
                "another ask over the same window and source: {outcome:?}"
            );
        }
        // A run whose opening record names no ask (every run before D-4950)
        // is never superseded, and a later run that names none supersedes
        // nothing.
        let legacy_then = |later: Vec<crate::audit::Record>| {
            let mut read = vec![opened_unasked(100, "kite", 20_000, 20_010), failure_at(110)];
            read.extend(later);
            interrupted_in(&numbered(read, 0)).map(|run| (run.ordinal, run.ask))
        };
        assert_eq!(
            legacy_then(vec![
                opened_at(500, "kite", 20_000, 20_010),
                closed_at(520, "kite", 20_000, 20_010),
            ]),
            Some((0, None)),
            "an older run with no ask is not shown to be the same pull"
        );
        assert_eq!(
            interrupted_then(vec![
                opened_unasked(500, "kite", 20_000, 20_010),
                closed_at(520, "kite", 20_000, 20_010),
            ]),
            Some(0),
            "a later run with no ask is not shown to be the same pull"
        );
        // Of two older runs of the same window and source, the one of the
        // same ask is superseded and the other is still reported.
        let read = vec![
            opened_asking(100, "kite", 20_000, 20_010, ASK ^ 1),
            failure_at(110),
            opened_at(200, "kite", 20_000, 20_010),
            failure_at(210),
            opened_at(300, "kite", 20_000, 20_010),
            closed_at(320, "kite", 20_000, 20_010),
        ];
        assert_eq!(
            interrupted_in(&numbered(read, 0)).map(|run| (run.ordinal, run.ask)),
            Some((0, Some(ASK ^ 1)))
        );
    }

    /// **WHAT IS DOUBTED BEFORE THE OPEN RUN IS ON THE DOCUMENT** (DB-B34,
    /// repair 3; follow-up 7): the records before its opening record that
    /// would not decode were named on the ERROR line only. And the count's
    /// words no longer claim only the newest 800 records were counted.
    #[test]
    fn the_damage_before_an_open_run_is_on_its_document() {
        let records = vec![
            opened_at(100, "kite", 20_000, 20_010),
            closed_at(120, "kite", 20_000, 20_010),
            opened_at(200, "kite", 20_020, 20_030),
            failure_at(210),
        ];
        let journal = journal_of("pullrun-r3-doubt-on-document", &records);
        damage(&journal, 1);
        let found = restart_scan(&journal);
        let run = found.interrupted().expect("the run at record 2 is open");
        assert_eq!((run.ordinal, run.undecodable_before), (2, 1));
        assert!(
            run.undecodable.contains("record 1 (not an audit record)"),
            "{run:?}"
        );
        assert!(
            run.words()
                .contains("1 record(s) before its opening record could not be decoded")
        );
        let doc: serde_json::Value =
            serde_json::from_str(&status_json(&Progress::default(), None, Some(&found)))
                .expect("valid JSON");
        assert_eq!(doc["interrupted"]["undecodableBefore"], 1);
        assert_eq!(doc["interrupted"]["olderUnread"], serde_json::Value::Null);
        // The damaged record is a close no record after it could belong
        // to: no closing record follows it that was paired by window alone.
        assert_eq!(doc["interrupted"]["newerMayBeOpen"], false);
        assert!(!run.newer_may_be_open);
        assert!(
            doc["interrupted"]["undecodable"]
                .as_str()
                .is_some_and(|named| named.contains("record 1")),
            "{doc}"
        );
        let mut floor = run.clone();
        floor.failures_whole = false;
        let words = floor.words();
        assert!(!words.contains("records were counted"), "{words}");
        assert!(words.contains("counted among the records read"), "{words}");
    }

    /// **DAMAGE BEFORE THE REPORTED RUN CAN HIDE A NEWER OPEN ONE, AND THE
    /// ANSWER SAYS SO** (DB-B34, repairs 4 and 6; the third check's probe
    /// `v62r3_p2`). A close that does not directly follow a record of the run
    /// it closes is paired with the newest open run of its scope, window and
    /// source; a record before it that would not decode may be the opening
    /// record of another run of that window and source that the close truly
    /// belongs to, and the run the walk closed would then be open.
    ///
    /// `[opened B s W1 (damaged)][opened Q t W2][opened R s W1][opened Y y W4]
    /// [closed y W4][closed s W1]`: whole, the last close pairs with R (the
    /// newest open run of `s W1`), R supersedes B (the same ask), and Q is
    /// reported. With record 0 undecodable the answer is the same run, Q, but
    /// the last close may be B's, and then R, newer than Q, is open: the
    /// answer says so. A
    /// pairing by position after the damage, or one before it, leaves no
    /// doubt; and a close that pairs with no run hides none.
    #[test]
    fn damage_before_the_reported_run_that_a_later_window_pairing_may_need_is_said() {
        let records = vec![
            opened_at(100, "s", 20_000, 20_010),
            opened_at(110, "t", 20_020, 20_030),
            opened_at(120, "s", 20_000, 20_010),
            opened_at(130, "y", 20_040, 20_050),
            closed_at(140, "y", 20_040, 20_050),
            closed_at(150, "s", 20_000, 20_010),
        ];
        let whole = journal_of("pullrun-r6-weak-whole", &records);
        let found = restart_scan(&whole);
        let run = found.interrupted().expect("Q is open");
        assert_eq!((run.ordinal, run.newer_may_be_open), (1, false));

        let journal = journal_of("pullrun-r6-weak-damaged", &records);
        damage(&journal, 0);
        let found = restart_scan(&journal);
        let run = found
            .interrupted()
            .expect("Q is open whatever record 0 was");
        assert_eq!((run.ordinal, run.undecodable_before), (1, 1));
        assert!(run.newer_may_be_open, "{run:?}");
        let words = run.words();
        assert!(
            words.contains("a run newer than this one may also have been left open"),
            "{words}"
        );
        assert!(words.contains("this one is open either way"), "{words}");
        let doc: serde_json::Value =
            serde_json::from_str(&status_json(&Progress::default(), None, Some(&found)))
                .expect("valid JSON");
        assert_eq!(doc["interrupted"]["newerMayBeOpen"], true);

        // A close by position after the damaged record: nothing lies between
        // it and its run, so no doubt.
        let records = vec![
            opened_at(100, "kite", 20_000, 20_010),
            opened_at(110, "probe", 20_020, 20_030),
            opened_at(120, "kite", 20_000, 20_010),
            closed_at(130, "kite", 20_000, 20_010),
        ];
        let journal = journal_of("pullrun-r6-beside-after-damage", &records);
        damage(&journal, 0);
        let run = restart_scan(&journal)
            .interrupted()
            .cloned()
            .expect("the probe run is open");
        assert_eq!((run.ordinal, run.newer_may_be_open), (1, false));

        // A pairing not by position that comes BEFORE the damaged record
        // cannot belong to it: no doubt about the newest run.
        let records = vec![
            opened_at(100, "s", 20_000, 20_010),
            opened_at(105, "z", 20_090, 20_099),
            closed_at(110, "s", 20_000, 20_010),
            failure_at(120),
            opened_at(130, "kite", 20_020, 20_030),
        ];
        let journal = journal_of("pullrun-r6-weak-before-damage", &records);
        damage(&journal, 3);
        let found = restart_scan(&journal);
        let run = found.interrupted().expect("the kite run is open");
        assert_eq!((run.ordinal, run.undecodable_before), (4, 1));
        assert!(!run.newer_may_be_open, "{run:?}");
        assert!(!run.words().contains("may also have been left open"));

        // A close that pairs with no run read (its run is the damaged record,
        // or no run at all) closes nothing, so it hides no run: no doubt.
        let records = vec![
            opened_at(100, "s", 20_000, 20_010),
            opened_at(110, "t", 20_020, 20_030),
            failure_at(120),
            closed_at(130, "s", 20_000, 20_010),
        ];
        let journal = journal_of("pullrun-r6-unpaired-after-damage", &records);
        damage(&journal, 0);
        let run = restart_scan(&journal)
            .interrupted()
            .cloned()
            .expect("t is open");
        assert_eq!((run.ordinal, run.newer_may_be_open), (1, false));
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
        // Its scope and its ask (DB-B34, repair 6): where it is run again, and
        // what a later run must name to supersede it.
        assert_eq!(parsed["interrupted"]["scope"], "spot");
        assert_eq!(
            parsed["interrupted"]["ask"],
            run.ask
                .map_or_else(String::new, |ask| format!("{ask:016x}"))
        );
        assert!(run.ask.is_some(), "{run:?}");
        assert!(
            parsed["interrupted"]["words"]
                .as_str()
                .is_some_and(|words| words.starts_with("INTERRUPTED: a pull run")
                    && words.contains("run the same pull again from /pull/spot")
                    && words.contains("names the same ask")),
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
                    // A failed leg that still wrote a bar: the run's own
                    // growth is its receipt's count (DB-B33, repair 4).
                    receipt_wrote(503, "FAILED", Some(1))
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
        assert_eq!(progress.bars_written, Some(1));
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
            bars_written: None,
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

/// Tests added by the cloud mutation pass (cloud/mut-api-b-20261016): each one
/// pins a behaviour a surviving mutant broke, or reaches a line no other test
/// reached. The helpers repeat the shapes `tests` uses, which are private to
/// that module.
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod cloud_mutant_tests {
    use super::*;
    use crate::audit::{Journal, RECORD_LEN, Record, Scope};

    const ASK: u64 = 0x00c1_0ad0_0000_0002;
    const GARBAGE: [u8; RECORD_LEN] = [0xFF; RECORD_LEN];

    fn at(secs: u64) -> std::time::SystemTime {
        std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs)
    }

    fn window(from: u32, to: u32) -> pull::session::Window {
        let day = |n| pull::session::Day::from_days(n).expect("a day");
        pull::session::Window::new(day(from), day(to)).expect("a window")
    }

    fn opened(secs: u64) -> Record {
        Record::opened(
            Scope::Spot,
            at(secs),
            "kite",
            window(20_000, 20_010),
            7,
            ASK,
        )
    }

    fn closed(secs: u64) -> Record {
        Record::refused(
            Scope::Spot,
            crate::audit::Outcome::Failed,
            at(secs),
            "kite",
            "closed",
        )
        .with_window(window(20_000, 20_010))
    }

    fn failure(secs: u64) -> Record {
        Record::member_failure(
            Scope::Spot,
            at(secs),
            "NIFTY",
            window(20_000, 20_000),
            "a gap",
        )
    }

    fn failures(from: u64, n: u64) -> Vec<[u8; RECORD_LEN]> {
        (0..n).map(|k| failure(from + k).image()).collect()
    }

    /// A journal of exactly these record images, with its index built over
    /// all of them.
    fn journal_of_images(name: &str, images: &[[u8; RECORD_LEN]]) -> Journal {
        let root = crate::scratch::path(&format!("pullrun-cloud-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let path = crate::audit::journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("audit/")).expect("mkdir");
        std::fs::write(&path, images.concat()).expect("write");
        let journal = Journal::at(&root);
        let built = journal.build_index().expect("the index builds");
        assert_eq!(built.records, images.len() as u64);
        journal
    }

    /// Appends images straight to the file, past the journal's own append,
    /// so the index is left behind exactly where it was.
    fn append_raw(journal: &Journal, images: &[[u8; RECORD_LEN]]) {
        use std::io::Write as _;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&journal.path)
            .expect("the journal opens")
            .write_all(&images.concat())
            .expect("the images land");
    }

    fn reason(found: &Restart) -> &str {
        match found {
            Restart::NotKnown(why) => why,
            other => panic!("NOT KNOWN was expected, and the scan said {other:?}"),
        }
    }

    fn lock(journal: &Journal) -> store::flock::Flock<std::fs::File> {
        store::flock::Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&journal.path)
                .expect("the journal handle"),
            journal.path.clone(),
        )
        .expect("nothing else holds it")
    }

    /// An F&O walk whose opening record names no ask and no window, with
    /// damage before it and an index that could not be read: every such
    /// field is `null` or named on the document, and the sentence says each.
    #[test]
    fn an_fno_walk_with_no_ask_or_window_is_written_with_nulls_and_its_doubts() {
        let run = Interrupted {
            ordinal: 3,
            opened_at_secs: 10,
            last_at_secs: 20,
            source: "dhan".to_owned(),
            scope: Scope::Fno,
            ask: None,
            window: None,
            failures: 2,
            failures_whole: true,
            note: "opened".to_owned(),
            undecodable_before: 1,
            undecodable: "record 1 (bad)".to_owned(),
            newer_may_be_open: false,
            older_unread: Some("no index".to_owned()),
            open_count: 1,
            also_open: Vec::new(),
            runs_unread: 0,
        };
        let mut out = String::new();
        run.json(&mut out);
        let doc: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["scope"], "fno");
        assert_eq!(doc["ask"], serde_json::Value::Null);
        assert_eq!(doc["from"], serde_json::Value::Null);
        assert_eq!(doc["to"], serde_json::Value::Null);
        assert_eq!(doc["olderUnread"], "no index");
        assert_eq!(doc["undecodableBefore"], 1);
        let words = run.words();
        assert!(words.contains("an expired-F&O walk was opened"), "{words}");
        assert!(words.contains("/pull/fno"), "{words}");
        assert!(words.contains("its opening record names no ask"), "{words}");
        assert!(words.contains("2 member failure(s)"), "{words}");
        assert!(
            words.contains(
                "1 record(s) before its opening record could not be decoded (record 1 (bad))"
            ),
            "{words}"
        );
        assert!(words.contains("(no index)"), "{words}");

        // No damage before it: no sentence about damage (kills `> 0` to
        // `>= 0`).
        let clean = Interrupted {
            undecodable_before: 0,
            undecodable: String::new(),
            older_unread: None,
            ..run
        };
        assert!(!clean.words().contains("could not be decoded"));
        assert!(!clean.words().contains("through the journal's index ("));
    }

    /// A feed whose store refused a write says so on the status document.
    #[test]
    fn a_feed_whose_store_is_unavailable_says_so_on_the_document() {
        let progress = Progress {
            feeds: vec![FeedReport {
                vendor: "dhan".to_owned(),
                store_unavailable: true,
                ..FeedReport::default()
            }],
            ..Progress::default()
        };
        let doc: serde_json::Value = serde_json::from_str(&progress.json()).expect("valid JSON");
        assert_eq!(doc["feeds"][0]["storeUnavailable"], true);
        assert_eq!(doc["feeds"][0]["credentialDead"], false);
    }

    /// A halted run's summary says what else ended it: the operator's stop,
    /// or the pass ceiling, and neither when it was neither.
    #[test]
    fn a_halted_summary_names_a_stop_or_the_ceiling_when_either_happened() {
        let halted = |passes, stopping| Progress {
            passes,
            retries: 1,
            stopping,
            feeds: vec![FeedReport {
                vendor: "dhan".to_owned(),
                credential_dead: true,
                last_error: Some("a dead token".to_owned()),
                ..FeedReport::default()
            }],
            ..Progress::default()
        };
        let stopped = run_summary(&halted(3, true), &[PassOutcome::Halted], Some(5));
        assert!(
            stopped.starts_with("INCOMPLETE after 3 pass(es); 5 bar(s) written"),
            "{stopped}"
        );
        assert!(stopped.contains("dhan: a dead token"), "{stopped}");
        assert!(
            stopped.ends_with(" The operator also pressed stop."),
            "{stopped}"
        );
        let ceiling = run_summary(&halted(MAX_PASSES, false), &[PassOutcome::Halted], None);
        assert!(ceiling.contains("an unknown number of bar(s)"), "{ceiling}");
        assert!(
            ceiling.ends_with(" The remaining work reached the pass ceiling."),
            "{ceiling}"
        );
        let neither = run_summary(&halted(3, false), &[PassOutcome::Halted], Some(0));
        assert!(
            neither.ends_with("Full basket coverage has not been verified."),
            "{neither}"
        );
    }

    /// A chain that died after its feed already named an error keeps that
    /// first error, and is still marked finished with its unattempted legs
    /// counted as skipped.
    #[tokio::test]
    async fn a_dead_chain_keeps_an_error_its_feed_already_named() {
        let site = site("dead-chain-named");
        let run = claim(&site);
        with_progress(&site, run, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: "dhan".to_owned(),
                legs: 3,
                legs_done: 1,
                doing: "dhan · 1min".to_owned(),
                last_error: Some("the first cause".to_owned()),
                ..FeedReport::default()
            }];
        });
        let dead = tokio::spawn(async { panic!("a chain that dies on purpose") })
            .await
            .expect_err("the task panicked");
        note_dead_chain(&site, run, 0, 2, &dead);
        let feed = observed(&site).feeds[0].clone();
        assert_eq!(feed.last_error.as_deref(), Some("the first cause"));
        assert!(feed.finished);
        assert_eq!(feed.doing, "");
        assert_eq!(feed.skipped, 1, "3 legs, 1 done, 1 interrupted");
    }

    /// A receipt's own refusal: a non-terminal outcome never overwrites the
    /// feed's first error, a terminal one does and names the halt, and the
    /// credential and store flags follow the outcome.
    #[test]
    fn a_receipt_refusal_overwrites_the_first_error_only_when_terminal() {
        let site = site("receipt-refusal-terminal");
        let run = claim(&site);
        with_progress(&site, run, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: "dhan".to_owned(),
                last_error: Some("the first cause".to_owned()),
                ..FeedReport::default()
            }];
        });
        let facts = vec![("Refused because", "a seat is held".to_owned())];
        let html = crate::render::receipt_page(&crate::render::Receipt {
            scope: "Spot pull",
            verdict: "REFUSED",
            reason: "synthetic",
            good: false,
            facts: &facts,
            footnote: "No vendor was contacted.",
        });
        let leg = Leg {
            route: Route::Spot,
            vendor: "dhan".to_owned(),
            dir: "1min".to_owned(),
            label: "dhan · 1min".to_owned(),
            body: String::new(),
        };
        let conflict = axum::http::StatusCode::CONFLICT;
        note_leg_failure(&site, run, 0, &leg, (conflict, &html), LegOutcome::Standing);
        let feed = observed(&site).feeds[0].clone();
        assert_eq!(feed.last_error.as_deref(), Some("the first cause"));
        assert!(!feed.store_unavailable && !feed.credential_dead);

        note_leg_failure(&site, run, 0, &leg, (conflict, &html), LegOutcome::Store);
        let feed = observed(&site).feeds[0].clone();
        let said = feed
            .last_error
            .expect("a terminal cause replaces the first");
        assert!(said.contains("a seat is held"), "{said}");
        assert!(
            said.contains("this feed is halted for the rest of this run"),
            "{said}"
        );
        assert!(
            said.contains("no journal record was written for a request refused at the door"),
            "{said}"
        );
        assert!(feed.store_unavailable);
        assert!(!feed.credential_dead);

        note_leg_failure(
            &site,
            run,
            0,
            &leg,
            (conflict, &html),
            LegOutcome::Credential,
        );
        assert!(observed(&site).feeds[0].credential_dead);

        // A standing outcome on a feed with no error yet names it as standing.
        with_progress(&site, run, |progress| progress.feeds[0].last_error = None);
        note_leg_failure(&site, run, 0, &leg, (conflict, &html), LegOutcome::Standing);
        let said = observed(&site).feeds[0].last_error.clone().expect("named");
        assert!(
            said.contains("no later pass of this run can change that"),
            "{said}"
        );
    }

    /// Without a refusal on the receipt, a terminal outcome still replaces an
    /// earlier transient error and sets its flag.
    #[test]
    fn a_terminal_outcome_without_a_refusal_replaces_the_first_error() {
        let site = site("terminal-no-refusal");
        let run = claim(&site);
        with_progress(&site, run, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: "dhan".to_owned(),
                last_error: Some("the first cause".to_owned()),
                ..FeedReport::default()
            }];
        });
        let leg = Leg {
            route: Route::Spot,
            vendor: "dhan".to_owned(),
            dir: "1min".to_owned(),
            label: "dhan · 1min".to_owned(),
            body: String::new(),
        };
        let status = axum::http::StatusCode::BAD_GATEWAY;
        note_leg_failure(
            &site,
            run,
            0,
            &leg,
            (status, "<p>no receipt</p>"),
            LegOutcome::Store,
        );
        let feed = observed(&site).feeds[0].clone();
        let said = feed.last_error.expect("replaced");
        assert!(
            said.starts_with("dhan · 1min answered HTTP 502: STORE UNAVAILABLE"),
            "{said}"
        );
        assert!(feed.store_unavailable);
    }

    /// The production request boundary sends an F&O leg to the F&O route,
    /// which refuses an empty form without asking any vendor.
    #[tokio::test]
    async fn an_fno_leg_reaches_the_fno_route() {
        let site = site("fno-leg-route");
        let leg = Leg {
            route: Route::Fno,
            vendor: "dhan".to_owned(),
            dir: "1min".to_owned(),
            label: "dhan · fno".to_owned(),
            body: String::new(),
        };
        let (status, body) = request_leg(site, leg).await;
        assert!(status.is_client_error(), "{status}: {body}");
        assert!(
            body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
            "{body}"
        );
    }

    /// A leg that answers as a transient failure and adds nothing is retried
    /// only after [`RETRY_WAIT`]: a stop pressed during that wait ends the run
    /// at the top of the loop with exactly one pass made (kills the retry
    /// branch's deletions and `passes < MAX_PASSES` turned around). The row
    /// ticker runs inside the wait.
    #[tokio::test]
    async fn a_retry_waits_before_the_next_pass_and_a_stop_in_the_wait_ends_it() {
        let held = site("retry-wait");
        let run = claim(&held);
        let stopper = {
            let site = Loaded::clone(&held);
            tokio::spawn(async move {
                tokio::time::sleep(core::time::Duration::from_secs(1)).await;
                with_progress(&site, run, |progress| progress.stopping = true);
            })
        };
        let started = std::time::Instant::now();
        let failing = |_site: Loaded, _leg: Leg| async {
            (
                axum::http::StatusCode::BAD_GATEWAY,
                crate::render::receipt_page(&crate::render::Receipt {
                    scope: "Spot pull",
                    verdict: "FAILED",
                    reason: "synthetic transient failure",
                    good: false,
                    facts: &[],
                    footnote: "No vendor was contacted.",
                }),
            )
        };
        let leg = Leg {
            route: Route::Spot,
            vendor: "dhan".to_owned(),
            dir: "1day".to_owned(),
            label: "dhan · 1day".to_owned(),
            body: String::new(),
        };
        // BOUNDED, so a run that never sees its stop FAILS here rather than
        // hanging the suite.
        tokio::time::timeout(
            core::time::Duration::from_mins(1),
            conduct_with(Loaded::clone(&held), run, vec![leg], failing),
        )
        .await
        .expect("the stop ends the run within one retry wait");
        stopper.await.expect("the stopper ran");
        let elapsed = started.elapsed();
        let seen = observed(&held);
        assert_eq!(seen.passes, 1, "no second pass before the wait ended");
        assert_eq!(seen.stop, Some(Stop::Operator));
        assert!(elapsed >= RETRY_WAIT, "the retry waited {elapsed:?}");
        assert!(seen.finished.is_some());
    }

    /// The journal's newest records cannot be read (the file will not open,
    /// though it can be measured): NOT KNOWN, by name. Root ignores modes, so
    /// this bites for a non-root user, as the suite is meant to run.
    #[test]
    fn newest_records_that_cannot_be_read_leave_the_answer_not_known() {
        use std::os::unix::fs::PermissionsExt as _;
        let mut images = vec![closed(100).image()];
        images.extend(failures(200, 10));
        let journal = journal_of_images("page-refused", &images);
        std::fs::set_permissions(&journal.path, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000");
        let found = restart_scan(&journal);
        std::fs::set_permissions(&journal.path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod back");
        let why = reason(&found);
        assert!(
            why.starts_with("the pull journal's newest records:"),
            "{why}"
        );
    }

    /// An index that covers EXACTLY the records before the scan is read as it
    /// is, with no build: here a held lock would make a build impossible, and
    /// the answer is still clean (kills `covered < scan_first` to `<=`).
    #[test]
    fn an_index_covering_exactly_the_records_before_the_scan_is_read_as_it_is() {
        let mut images = vec![closed(100).image()];
        images.extend(failures(200, 99));
        let journal = journal_of_images("index-exact", &images);
        append_raw(&journal, &failures(1_000, INTERRUPTED_SCAN));
        let squatter = lock(&journal);
        let found = restart_scan_waiting(&journal, 0);
        drop(squatter);
        assert_eq!(found, Restart::Clean);
    }

    /// An index behind the records before the scan is caught up here and
    /// then read: the answer is the one a whole index gives, and the index
    /// now covers the journal.
    #[test]
    fn an_index_behind_the_scan_is_caught_up_and_then_read() {
        let mut images = vec![opened(100).image()];
        images.extend(failures(200, 99));
        let journal = journal_of_images("index-behind", &images);
        append_raw(&journal, &failures(1_000, INTERRUPTED_SCAN + 1));
        let found = restart_scan(&journal);
        assert_eq!(
            found
                .interrupted()
                .map(|run| (run.ordinal, run.failures_whole)),
            Some((0, false)),
            "{found:?}"
        );
        let records = 100 + INTERRUPTED_SCAN + 1;
        assert_eq!(
            journal
                .read_index(records)
                .expect("the index reads")
                .covered,
            records
        );
    }

    /// With a held lock and the index absent, a reader that finds the index
    /// back in place between two waits answers through it (the `older_runs`
    /// retried after a wait).
    #[test]
    fn an_index_that_reappears_while_the_lock_is_held_is_read_after_a_wait() {
        let mut images = vec![opened(100).image()];
        images.extend(failures(200, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("index-reappears", &images);
        let index = journal.index_dir();
        let aside = index.with_extension("aside");
        std::fs::rename(&index, &aside).expect("moved aside");
        let squatter = lock(&journal);
        let restorer = std::thread::spawn({
            let (index, aside) = (index.clone(), aside.clone());
            move || {
                std::thread::sleep(core::time::Duration::from_millis(350));
                std::fs::rename(&aside, &index).expect("moved back");
            }
        });
        let found = restart_scan_waiting(&journal, 50);
        drop(squatter);
        restorer.join().expect("the restorer ran");
        assert_eq!(
            found.interrupted().map(|run| run.ordinal),
            Some(0),
            "{found:?}"
        );
    }

    /// An index caught up here that still cannot be read, for a reason other
    /// than the first, says both: what was wrong, that it was caught up, and
    /// what is wrong now.
    #[test]
    fn a_catch_up_that_meets_a_new_fault_names_both() {
        let mut images = vec![closed(100).image()];
        images.extend(failures(200, 100));
        let journal = journal_of_images("index-new-fault", &images);
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entry is overwritten");
        append_raw(&journal, &failures(1_000, INTERRUPTED_SCAN + 50));
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(
            why.contains(
                "the index covers the first 101 records and the scan begins at record 151"
            ),
            "{why}"
        );
        assert!(
            why.contains(
                "it was built or caught up here, and then: 1 of the index's entries cannot be read"
            ),
            "{why}"
        );
    }

    /// More than [`INTERRUPTED_RUNS`] run records before the scan: the walk
    /// begins at the oldest of the 64 it reads, so a damaged record before
    /// that is outside it and the answer is clean (kills `below >
    /// INTERRUPTED_RUNS` turned to `<` or `==`, and the faults' lower bound
    /// turned around).
    #[test]
    fn damage_older_than_the_runs_the_walk_reads_is_outside_it() {
        let mut images = vec![failure(10).image(), GARBAGE, failure(11).image()];
        for pair in 0..35_u64 {
            images.push(opened(1_000 + pair * 10).image());
            images.push(closed(1_005 + pair * 10).image());
        }
        images.extend(failures(5_000, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("runs-past-cap", &images);
        assert_eq!(restart_scan(&journal), Restart::Clean);
    }

    /// Exactly [`INTERRUPTED_RUNS`] run records before the scan: every one
    /// was read, so the walk begins at the journal's first record and a
    /// damaged record before the first run record is inside it (kills `>` to
    /// `>=`).
    #[test]
    fn with_exactly_64_runs_before_the_scan_the_walk_begins_at_record_0() {
        let mut images = vec![failure(10).image(), GARBAGE, failure(11).image()];
        for pair in 0..32_u64 {
            images.push(opened(1_000 + pair * 10).image());
            images.push(closed(1_005 + pair * 10).image());
        }
        images.extend(failures(5_000, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("runs-at-cap", &images);
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(
            why.starts_with("1 of the journal records the scan read"),
            "{why}"
        );
        assert!(why.contains("record 1 ("), "{why}");
    }

    /// A damaged record AT the journal's first record is inside a walk that
    /// begins there (kills the faults' lower bound `<` turned to `<=` or
    /// `==`).
    #[test]
    fn damage_at_record_0_is_inside_a_walk_that_begins_there() {
        let mut images = vec![GARBAGE, closed(100).image()];
        images.extend(failures(200, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("damage-at-0", &images);
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(why.contains("record 0 ("), "{why}");
    }

    /// More than [`INTERRUPTED_RUNS`] faults the index filed inside the
    /// walk: the newest 64 are read again and the older ones are reported as
    /// the index filed them.
    #[test]
    fn faults_past_the_cap_are_reported_as_the_index_filed_them() {
        let mut images = vec![GARBAGE; 70];
        images.push(closed(100).image());
        images.extend(failures(200, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("faults-past-cap", &images);
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(
            why.starts_with("70 of the journal records the scan read"),
            "{why}"
        );
        assert!(
            why.contains(
                "record 0 (the journal's index files it as a record that would not decode)"
            ),
            "{why}"
        );
        assert!(why.contains("and 67 more, the newest record 69"), "{why}");
    }

    /// The pullrun test site shape: a universe and a store root of its own.
    fn site(name: &str) -> Loaded {
        let store = crate::scratch::path(&format!("pullrun-cloud-site-{name}"));
        let _ = std::fs::remove_dir_all(&store);
        std::fs::create_dir_all(store.join("manifest")).expect("mkdir");
        let masters = crate::scratch::path(&format!("pullrun-cloud-masters-{name}"));
        std::fs::create_dir_all(&masters).expect("mkdir");
        std::fs::write(
            masters.join("groww_instruments.csv"),
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n",
        )
        .expect("write");
        Loaded::new(Site::load(&masters, &store))
    }

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

    fn observed(site: &Site) -> Progress {
        site.run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or_default()
    }

    /// The leg encoding the page writes: `route|vendor|dir|label|payload`,
    /// each part's `%`, `&`, `=`, `|` and `+` escaped, twice over.
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

    /// A leg whose payload names more distinct fields than the form
    /// middleware's own bound refuses the whole run by name, and the bound
    /// itself is admitted (`FormKeys::TooMany`).
    #[test]
    fn a_leg_whose_payload_names_too_many_fields_refuses_the_run() {
        let leg = |distinct: usize| {
            use std::fmt::Write as _;
            let mut body = String::from("vendor=dhan&granularity=1day");
            for n in 2..distinct {
                let _infallible = write!(body, "&k{n}=v");
            }
            let joined = format!("/pull/spot|dhan|1day|wide|{}", enc(&body));
            format!("leg={}", enc(&joined))
        };
        let bound = crate::server::MAX_DISTINCT_FORM_KEYS;
        assert_eq!(
            legs_from(&leg(bound)).expect("the bound is admitted").len(),
            1
        );
        match legs_from(&leg(bound + 1)) {
            Err(Refusal::Malformed(why)) => assert!(
                why.ends_with(&format!(
                    "(its form names more than {bound} distinct fields)"
                )),
                "{why}"
            ),
            other => panic!("a payload past the bound must refuse the run: {other:?}"),
        }
    }

    /// A leg failure or a dead chain addressed to a feed the document does
    /// not hold changes nothing and does not panic.
    #[tokio::test]
    async fn an_edit_to_a_feed_the_document_does_not_hold_changes_nothing() {
        let site = site("edit-past-feeds");
        let run = claim(&site);
        let before = observed(&site);
        let leg = Leg {
            route: Route::Spot,
            vendor: "dhan".to_owned(),
            dir: "1day".to_owned(),
            label: "dhan · 1day".to_owned(),
            body: String::new(),
        };
        let status = axum::http::StatusCode::BAD_GATEWAY;
        note_leg_failure(
            &site,
            run,
            3,
            &leg,
            (status, "<p>no receipt</p>"),
            LegOutcome::Store,
        );
        let facts = vec![("Refused because", "a seat is held".to_owned())];
        let refused = crate::render::receipt_page(&crate::render::Receipt {
            scope: "Spot pull",
            verdict: "REFUSED",
            reason: "synthetic",
            good: false,
            facts: &facts,
            footnote: "No vendor was contacted.",
        });
        note_leg_failure(&site, run, 3, &leg, (status, &refused), LegOutcome::Store);
        let dead = tokio::spawn(async { panic!("a chain that dies on purpose") })
            .await
            .expect_err("the task panicked");
        note_dead_chain(&site, run, 3, 1, &dead);
        assert_eq!(observed(&site), before);
        assert!(observed(&site).feeds.is_empty());
    }

    /// A synthetic spot receipt: `verdict`, and `wrote` bars under the spot
    /// landing's label.
    fn receipt_wrote(status: u16, verdict: &str, wrote: u64) -> (axum::http::StatusCode, String) {
        let facts = vec![(BARS_WRITTEN_FACTS[0], wrote.to_string())];
        (
            axum::http::StatusCode::from_u16(status).expect("HTTP status"),
            crate::render::receipt_page(&crate::render::Receipt {
                scope: "Spot pull",
                verdict,
                reason: "synthetic coordinator receipt",
                good: verdict == "STORED",
                facts: &facts,
                footnote: "No vendor was contacted.",
            }),
        )
    }

    fn spot_leg(vendor: &str) -> Leg {
        Leg {
            route: Route::Spot,
            vendor: vendor.to_owned(),
            dir: "1day".to_owned(),
            label: format!("{vendor} · 1day"),
            body: String::new(),
        }
    }

    /// A run whose every pass stores more bars runs EXACTLY [`MAX_PASSES`]
    /// passes, and when its last pass leaves a leg owed it stops at the
    /// ceiling at once, with no retry wait for a pass that can never run.
    /// Each feed's document names how many legs it carries. Kills `passes <
    /// MAX_PASSES` turned to `<=` (a 401st request), the retry wait's own `<`
    /// turned to `<=` (a twenty-second sleep), the idle flag's `>=` turned
    /// to `<` (the stop would not be the ceiling) and the deleted `legs` field.
    #[tokio::test]
    async fn a_run_that_grows_every_pass_stops_at_the_pass_ceiling_without_a_last_wait() {
        let held = site("pass-ceiling");
        let run = claim(&held);
        let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = std::sync::Arc::clone(&asked);
        let request = move |_site: Loaded, _leg: Leg| {
            let nth = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            async move {
                if nth < MAX_PASSES as usize {
                    receipt_wrote(200, "STORED", 1)
                } else {
                    receipt_wrote(502, "FAILED", 0)
                }
            }
        };
        let started = std::time::Instant::now();
        tokio::time::timeout(
            core::time::Duration::from_secs(15),
            conduct_with(Loaded::clone(&held), run, vec![spot_leg("dhan")], request),
        )
        .await
        .expect("no retry wait after the last allowed pass");
        let elapsed = started.elapsed();
        let seen = observed(&held);
        assert_eq!(
            asked.load(std::sync::atomic::Ordering::SeqCst),
            MAX_PASSES as usize
        );
        assert_eq!(seen.passes, MAX_PASSES);
        assert_eq!(seen.stop, Some(Stop::Ceiling), "{:?}", seen.finished);
        assert!(elapsed < RETRY_WAIT, "{elapsed:?}");
        assert_eq!(seen.feeds.len(), 1);
        assert_eq!(seen.feeds[0].legs, 1, "the document names the feed's legs");
        assert_eq!(seen.bars_written, Some(u64::from(MAX_PASSES) - 1));
    }

    /// Two feeds render as two comma-separated objects that parse (kills the
    /// feeds separator's `nth > 0` turned to `>=`).
    #[test]
    fn two_feeds_render_as_valid_json() {
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
        let doc: serde_json::Value = serde_json::from_str(&progress.json()).expect("valid JSON");
        assert_eq!(doc["feeds"].as_array().map(Vec::len), Some(2));
        assert_eq!(doc["feeds"][1]["vendor"], "groww");
    }

    /// No feed with standing failures, no standing note (kills `standing >
    /// 0` turned to `>=`).
    #[test]
    fn a_feed_with_no_standing_failures_is_not_named_as_standing() {
        let progress = Progress {
            feeds: vec![FeedReport {
                vendor: "dhan".to_owned(),
                standing: 0,
                ..FeedReport::default()
            }],
            ..Progress::default()
        };
        assert_eq!(standing_note(&progress), "");
        let standing = Progress {
            feeds: vec![FeedReport {
                vendor: "dhan".to_owned(),
                standing: 1,
                ..FeedReport::default()
            }],
            ..Progress::default()
        };
        assert!(standing_note(&standing).contains("dhan (1 leg(s))"));
    }

    /// A chain that died having attempted only the legs it finished counts
    /// none as interrupted (kills `attempted > legs_done` turned to `>=`).
    #[tokio::test]
    async fn a_dead_chain_whose_attempts_all_returned_skips_the_rest() {
        let site = site("dead-chain-even");
        let run = claim(&site);
        with_progress(&site, run, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: "dhan".to_owned(),
                legs: 3,
                legs_done: 1,
                ..FeedReport::default()
            }];
        });
        let dead = tokio::spawn(async { panic!("a chain that dies on purpose") })
            .await
            .expect_err("the task panicked");
        note_dead_chain(&site, run, 0, 1, &dead);
        assert_eq!(
            observed(&site).feeds[0].skipped,
            2,
            "3 legs, 1 done, none interrupted"
        );
    }

    /// An opening record with no window is a run with no window, and one
    /// whose window starts on day 0 keeps it (kills each `!= 0` turned to
    /// `== 0` and the `||` turned to `&&`).
    #[test]
    fn a_run_record_with_no_window_is_read_as_having_none() {
        let mut bare = opened(100);
        bare.from_days = 0;
        bare.to_days = 0;
        let open = interrupted_in(&[(0, bare)]).expect("open");
        assert_eq!(open.window, None);
        let mut epoch = opened(100);
        epoch.from_days = 0;
        epoch.to_days = 5;
        let open = interrupted_in(&[(0, epoch)]).expect("open");
        assert_eq!(open.window, Some((0, 5)));
    }

    /// A journal the scan reads whole needs no index: with the index gone
    /// and `audit/` taking no write it is still CLEAN, never NOT KNOWN
    /// (kills `scan_first > 0` turned to `>=`). Root ignores modes, so this
    /// bites for a non-root user, as the suite is meant to run.
    #[test]
    fn a_journal_read_whole_by_the_scan_needs_no_index() {
        use std::os::unix::fs::PermissionsExt as _;
        let mut images = vec![opened(100).image(), closed(120).image()];
        images.extend(failures(200, 10));
        let journal = journal_of_images("whole-no-index", &images);
        let index = journal.index_dir();
        std::fs::rename(&index, index.with_extension("aside")).expect("moved aside");
        let audit = index.parent().expect("audit/").to_path_buf();
        std::fs::set_permissions(&audit, std::fs::Permissions::from_mode(0o555)).expect("chmod");
        let found = restart_scan_waiting(&journal, 0);
        std::fs::set_permissions(&audit, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        assert_eq!(found, Restart::Clean);
    }

    /// With the index unreadable and only member failures in the scan, the
    /// reason says the scan holds no run record (kills the `settled` test's
    /// `==` turned to `!=` and `&&` turned to `||`).
    #[test]
    fn a_scan_of_member_failures_alone_holds_no_run_record() {
        let mut images = vec![opened(100).image()];
        images.extend(failures(200, INTERRUPTED_SCAN + 50));
        let journal = journal_of_images("failures-alone", &images);
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entry is overwritten");
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(why.contains("records hold no run record"), "{why}");
    }

    /// A run opened at EXACTLY the scan's first record was read with every
    /// record after it, so with the index unreadable it is still reported
    /// open, with the older runs named unread (kills `run.ordinal <
    /// scan_first` turned to `<=`).
    #[test]
    fn a_run_opened_at_the_scans_first_record_is_open_though_the_index_is_unreadable() {
        let mut images = failures(10, 10);
        images.push(opened(100).image());
        images.extend(failures(200, INTERRUPTED_SCAN - 1));
        let journal = journal_of_images("open-at-scan-first", &images);
        let runs = journal.index_dir().join("runs");
        let mut bytes = std::fs::read(&runs).expect("the runs file");
        bytes.fill(0xFF);
        std::fs::write(&runs, bytes).expect("the entry is overwritten");
        let found = restart_scan(&journal);
        let run = found.interrupted().expect("it is open");
        assert_eq!(run.ordinal, 10);
        assert!(run.older_unread.is_some());
    }

    /// A run record AT the scan's first record is the scan's, not one of the
    /// [`INTERRUPTED_RUNS`] the index supplies: with exactly 64 run records
    /// before the scan the walk still begins at record 0, so a damaged
    /// record between the first two is inside it and named (kills the index
    /// partition's `ordinal < scan_first` turned to `<=`, which would take
    /// the scan's own run record as the 64th and push the walk past the
    /// damage).
    #[test]
    fn a_run_record_at_the_scans_first_record_does_not_shift_the_walk() {
        let mut images = vec![closed(100).image(), GARBAGE];
        images.extend((0..63).map(|k| closed(110 + k).image()));
        images.push(closed(500).image());
        images.extend(failures(1_000, INTERRUPTED_SCAN - 1));
        assert_eq!(
            images.len() as u64 - INTERRUPTED_SCAN,
            65,
            "the scan begins at 65"
        );
        let journal = journal_of_images("run-at-scan-first", &images);
        let found = restart_scan(&journal);
        let why = reason(&found);
        assert!(
            why.starts_with("1 of the journal records the scan read"),
            "{why}"
        );
        assert!(why.contains("record 1 ("), "{why}");
    }

    /// One leg of `route` on `dir` for `vendor`.
    fn leg_of(route: Route, vendor: &str, dir: &str) -> Leg {
        Leg {
            route,
            vendor: vendor.to_owned(),
            dir: dir.to_owned(),
            label: format!("{vendor} · {dir}"),
            body: String::new(),
        }
    }

    /// `legs` through `run_chain` on feed slot `nth`, each answered by
    /// `answer` in turn; answers how many legs were attempted, and the
    /// chain's verdict.
    async fn chain_over(
        site: &Loaded,
        run: u64,
        nth: usize,
        legs: Vec<Leg>,
        answers: Vec<(axum::http::StatusCode, String)>,
    ) -> (usize, PassOutcome) {
        let replies = std::sync::Arc::new(std::sync::Mutex::new(answers.into_iter()));
        let attempted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let checkpoints = legs
            .iter()
            .map(|_| std::sync::atomic::AtomicU8::new(OWED))
            .collect();
        let outcome = run_chain(
            Loaded::clone(site),
            run,
            nth,
            legs,
            checkpoints,
            std::sync::Arc::clone(&attempted),
            move |_site: Loaded, _leg: Leg| {
                let next = replies
                    .lock()
                    .expect("the replies")
                    .next()
                    .expect("one answer per attempted leg");
                async move { next }
            },
        )
        .await;
        (attempted.load(std::sync::atomic::Ordering::SeqCst), outcome)
    }

    /// **A failed derivative leg holds back no later leg.** Only a failed
    /// SPOT rung holds back what is behind it (DB-B26): a failed F&O leg
    /// leaves the next F&O leg of the chain owed and asked. Covers the false
    /// side of `run_chain`'s `leg.route == Route::Spot` (pullrun.rs:3733);
    /// kills it turned to `!=`, under which the failed futures leg would
    /// hold the options leg back.
    #[tokio::test]
    async fn a_failed_derivative_leg_holds_back_no_later_leg() {
        let site = site("fno-fail-holds-nothing");
        let run = claim(&site);
        with_progress(&site, run, |progress| {
            progress.feeds = vec![FeedReport {
                vendor: "dhan".to_owned(),
                legs: 2,
                ..FeedReport::default()
            }];
        });
        let (attempted, outcome) = chain_over(
            &site,
            run,
            0,
            vec![
                leg_of(Route::Fno, "dhan", "futures"),
                leg_of(Route::Fno, "dhan", "options"),
            ],
            vec![
                receipt_wrote(502, "FAILED", 0),
                receipt_wrote(200, "STORED", 3),
            ],
        )
        .await;
        assert_eq!(attempted, 2, "the options leg was still asked");
        assert_eq!(outcome, PassOutcome::Retry, "the failed leg is retried");
        assert_eq!(observed(&site).feeds[0].legs_done, 2);
    }

    /// **A chain whose feed the document does not hold still runs, and
    /// writes nothing into another feed's row.** The slot is past the
    /// document's feeds, so every per-feed edit (the doing label, the legs
    /// done, the standing count and the end-of-chain skip) finds no row; the
    /// run-wide figures still move. Covers the `None` sides of
    /// `progress.feeds.get_mut(nth)` in `run_chain` (pullrun.rs:3699, 3708,
    /// 3727, 3746).
    #[tokio::test]
    async fn a_chain_whose_feed_the_document_does_not_hold_still_runs() {
        let site = site("chain-past-feeds");
        let run = claim(&site);
        with_progress(&site, run, |progress| progress.bars_written = Some(0));
        let standing = {
            let facts = vec![(RETRY_CLASS_FACT, RetryClass::Standing.word().to_owned())];
            (
                axum::http::StatusCode::BAD_GATEWAY,
                crate::render::receipt_page(&crate::render::Receipt {
                    scope: "Spot pull",
                    verdict: "FAILED",
                    reason: "synthetic standing receipt",
                    good: false,
                    facts: &facts,
                    footnote: "No vendor was contacted.",
                }),
            )
        };
        assert_eq!(
            leg_outcome(standing.0, &standing.1),
            LegOutcome::Standing,
            "the premise"
        );
        let (attempted, outcome) = chain_over(
            &site,
            run,
            4,
            vec![spot_leg("dhan"), leg_of(Route::Spot, "dhan", "1min")],
            vec![standing, receipt_wrote(200, "STORED", 7)],
        )
        .await;
        assert_eq!((attempted, outcome), (2, PassOutcome::Clean));
        let progress = observed(&site);
        assert!(progress.feeds.is_empty(), "no row was made for slot 4");
        assert_eq!(progress.bars_written, Some(7), "the run-wide count moved");
    }

    /// **A non-success page that names a dead credential halts the feed,
    /// whatever its status.** A 502 receipt carrying the vendor's own 401
    /// sentence is a credential fault; the same sentence on a 200 is read
    /// by the receipt's badge first. Covers the right side of
    /// `leg_outcome`'s first `||` (pullrun.rs:3442).
    #[test]
    fn a_failed_page_naming_a_dead_credential_halts_whatever_its_status() {
        let page = "<p>the vendor refused with status 401</p>";
        assert_eq!(
            leg_outcome(axum::http::StatusCode::BAD_GATEWAY, page),
            LegOutcome::Credential
        );
        let (status, stored) = receipt_wrote(200, "STORED", 1);
        assert_eq!(leg_outcome(status, &stored), LegOutcome::Stored);
        let (_, failed) = receipt_wrote(502, "FAILED", 0);
        assert_eq!(
            leg_outcome(axum::http::StatusCode::BAD_GATEWAY, &failed),
            LegOutcome::Retry,
            "a failed page naming no credential is not one"
        );
    }

    /// **Each store vendor is named once, and a wire no feed carries names
    /// none.** Covers both `None`/duplicate sides of `own_vendors`
    /// (pullrun.rs:3251, 3252).
    #[test]
    fn own_vendors_names_each_store_vendor_once_and_no_unknown_wire() {
        let groww = pull::vendor::Feed::Groww.wire().to_owned();
        let dhan = pull::vendor::Feed::Dhan.wire().to_owned();
        let groups = vec![
            ("no-such-wire".to_owned(), Vec::new()),
            (groww.clone(), Vec::new()),
            (dhan, Vec::new()),
            (groww, Vec::new()),
        ];
        assert_eq!(
            own_vendors(&groups),
            [
                brutex_core::vendor::Vendor::Groww,
                brutex_core::vendor::Vendor::Dhan
            ]
        );
        assert!(own_vendors(&[]).is_empty());
    }

    /// **An edit with no run in the slot changes nothing and is not made.**
    /// Covers the `None` side of `with_progress`'s `held.as_mut()`
    /// (pullrun.rs:3287).
    #[test]
    fn an_edit_with_no_run_in_the_slot_is_not_made() {
        let site = site("edit-no-run");
        let made = std::cell::Cell::new(false);
        with_progress(&site, 1, |_progress| made.set(true));
        assert!(!made.get(), "nothing holds the slot, so nothing is edited");
        assert!(
            site.run
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_none()
        );
    }

    /// **A calendar of record that has ended says so on the run's
    /// document.** Seen from a day past its last sourced day, the horizon
    /// has no day left and `ended` is true. Covers `status_json`'s
    /// `ended` true arm (pullrun.rs:2880).
    #[test]
    fn a_calendar_of_record_that_has_ended_says_so() {
        let past = pull::calendar::record_horizon(0).last_day + 10;
        let doc: serde_json::Value =
            serde_json::from_str(&status_json(&Progress::default(), Some(past), None))
                .expect("valid JSON");
        assert_eq!(doc["calendar"]["ended"], true, "{doc}");
        assert_eq!(doc["calendar"]["endsSoon"], true, "{doc}");
        assert_eq!(doc["calendar"]["daysLeft"], -10, "{doc}");
    }
}

/// Tests added by the cloud mutation pass of 17 Oct (cloud/mut-api-b-20261017)
/// for the restart answer's renderers: every arm of `AlsoOpen` and
/// `Guessed` in words and on the document, driven by value, since a journal
/// that reaches each arm through the walk is not needed to prove what each
/// arm writes.
#[cfg(test)]
mod cloud_render_tests {
    use super::*;
    use crate::audit::Scope;

    fn other(ordinal: u64, scope: Scope, doubt: Option<&'static str>) -> AlsoOpen {
        AlsoOpen {
            ordinal,
            opened_at_secs: 10,
            last_at_secs: 30,
            source: "dhan".to_owned(),
            scope,
            ask: None,
            window: None,
            failures: 3,
            failures_whole: false,
            doubt,
        }
    }

    /// **Another run left open with no window and no ask is said so, and
    /// written with nulls.** An expired-F&O walk, a count that is a floor and
    /// a doubt, each in its own words. Covers `AlsoOpen::clause`'s F&O scope
    /// and no-window arms and `AlsoOpen::json`'s F&O scope, null ask and
    /// null window arms (pullrun.rs:761, 764, 797, 802, 811).
    #[test]
    fn another_open_walk_with_no_window_or_ask_is_said_and_written_with_nulls() {
        let walk = other(4, Scope::Fno, Some("a record after it would not decode"));
        assert_eq!(
            walk.clause(),
            "record 4 (an expired-F&O walk, no window, source \"dhan\", no ask, opened at 10 s \
             and last read at 30 s since the epoch, at least 3 member failure(s); it may have \
             been closed: a record after it would not decode)"
        );
        let mut out = String::new();
        walk.json(&mut out);
        let doc: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["scope"], "fno");
        assert_eq!(doc["ask"], serde_json::Value::Null);
        assert_eq!(doc["from"], serde_json::Value::Null);
        assert_eq!(doc["to"], serde_json::Value::Null);
        assert_eq!(doc["failures"], 3);
        assert_eq!(doc["failuresWhole"], false);
        assert_eq!(doc["mayBeClosed"], "a record after it would not decode");
    }

    /// **A guessed answer with no reading says every half it holds, and
    /// writes every list whole.** Two guessed runs, two sure, two doubted,
    /// two counted pairings (one spot, one F&O) and two runs counted and not
    /// named: the words name each, the document carries each list in order
    /// with its separators, and `reading` is null. Covers `Guessed::words`'
    /// guessed-runs and unnamed sentences and `Guessed::json`'s null reading,
    /// list separators and spot scope (pullrun.rs:1220, 1232-1238, 1265,
    /// 1278-1293, 1310).
    #[test]
    fn a_guessed_answer_with_no_reading_says_and_writes_every_list() {
        let guessed = Guessed {
            why: "a guess decides it".to_owned(),
            reading: None,
            guessed_runs: vec![3, 5],
            sure: vec![other(7, Scope::Spot, None), other(9, Scope::Fno, None)],
            doubted: vec![
                other(11, Scope::Spot, Some("one")),
                other(13, Scope::Spot, Some("two")),
            ],
            unnamed: 2,
            left_open: vec![
                LeftOpen {
                    close: 20,
                    runs: vec![17, 15],
                    left_open: 1,
                    scope: Scope::Spot,
                    source: "kite".to_owned(),
                },
                LeftOpen {
                    close: 22,
                    runs: vec![19, 18, 16],
                    left_open: 2,
                    scope: Scope::Fno,
                    source: "dhan".to_owned(),
                },
            ],
        };
        assert_eq!(guessed.at_least_open(), 5, "two sure and three counted");
        let words = guessed.words();
        assert!(
            words.starts_with(
                "NOT KNOWN which pull runs were left open, though at least 5 run(s) were: \
                 a guess decides it."
            ),
            "{words}"
        );
        assert!(
            words.contains(
                " A guess names the run(s) opened at records 3, 5: each is open or closed \
                 whichever way its guess went."
            ),
            "{words}"
        );
        assert!(
            words.contains(&format!(
                " 2 more run(s) of the walk's reading have no closing record and are counted, \
                 not named (a document names {OPEN_NAMED}); `/audit` lists them."
            )),
            "{words}"
        );
        assert!(
            !words.contains("older run record(s)"),
            "no reading, so no run record is said to be unread: {words}"
        );
        assert!(
            words.ends_with(" Nothing else is claimed; `/audit` lists the journal's runs."),
            "{words}"
        );

        let mut keys = String::from("{\"first\":0");
        guessed.json(&mut keys);
        keys.push('}');
        let doc: serde_json::Value = serde_json::from_str(&keys).expect("valid JSON");
        assert_eq!(doc["reading"], serde_json::Value::Null);
        assert_eq!(doc["guessedRuns"], serde_json::json!([3, 5]));
        let ordinals = |key: &str| -> Vec<u64> {
            doc[key]
                .as_array()
                .expect("a list")
                .iter()
                .map(|run| run["ordinal"].as_u64().expect("an ordinal"))
                .collect()
        };
        assert_eq!(ordinals("sureOpen"), [7, 9]);
        assert_eq!(ordinals("doubtedOpen"), [11, 13]);
        let left = doc["leftOpen"].as_array().expect("a list");
        assert_eq!(left.len(), 2);
        assert_eq!(left[0]["scope"], "spot");
        assert_eq!(left[0]["runs"], serde_json::json!([17, 15]));
        assert_eq!(left[1]["scope"], "fno");
        assert_eq!(left[1]["leftOpen"], 2);
        assert_eq!(left[1]["source"], "dhan");
        assert_eq!(doc["atLeastOpen"], 5);
        assert_eq!(doc["unnamed"], 2);
    }
}
