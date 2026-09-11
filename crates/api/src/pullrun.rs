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
//! (four), not by the number of legs. The pass loop holds one `Progress` and
//! one checkpoint per leg; nothing accumulates per pass.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use crate::census;
use crate::server::{Loaded, Site, percent_decode};

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
    /// The store's row count when the run started.
    pub rows_at_start: u64,
    /// The store's row count as of the last pass.
    pub rows_now: u64,
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
}

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
            ..Self::default()
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
        out.push_str(&self.rows_at_start.to_string());
        out.push_str(",\"rowsNow\":");
        out.push_str(&self.rows_now.to_string());
        out.push_str(",\"stopping\":");
        out.push_str(if self.stopping { "true" } else { "false" });
        out.push_str(",\"finished\":");
        match &self.finished {
            Some(word) => out.push_str(&quote_for_json(word)),
            None => out.push_str("null"),
        }
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
/// [`Refusal::Malformed`] naming the first leg that could not be read.
///
/// # Cost
///
/// One pass over the body. O(n) in its length and nothing worse -- there is no
/// per-leg scan of the legs already read.
pub fn legs_from(body: &str) -> Result<Vec<Leg>, Refusal> {
    let mut legs = Vec::new();
    for field in body.split('&') {
        let Some(("leg", raw)) = field.split_once('=') else {
            continue;
        };
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
        legs.push(Leg {
            route,
            vendor: vendor.to_owned(),
            dir: dir.to_owned(),
            label: label.to_owned(),
            body: percent_decode(payload),
        });
    }
    if legs.is_empty() {
        return Err(Refusal::NothingAsked);
    }
    Ok(legs)
}

/// Groups legs by vendor, keeping the order the feeds were ticked, and sorts
/// each group onto the ladder.
///
/// # Cost
///
/// The group lookup scans the vendors found so far, which is bounded by the
/// number of feeds this build has rather than by the number of legs -- four,
/// not four thousand. The sort is per group and `sort_by_key` is stable, so two
/// legs on one rung keep the order the operator ticked them in.
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
/// One manifest read per vendor per pass — four reads, not four per leg. It is
/// deliberately outside the per-leg path.
pub(crate) fn rows_now(site: &Site) -> u64 {
    let (censuses, _) = crate::server::census_now(site);
    censuses
        .iter()
        .filter_map(census::VendorCensus::counters)
        .map(|(_months, rows, _entries)| rows)
        .sum()
}

/// Edits the live progress, if a run still owns the slot.
///
/// A closure rather than a returned guard, so the lock cannot be held across an
/// `.await` by accident — which is the one way a `std::sync::Mutex` here could
/// stall the whole fan-out.
fn with_progress<F: FnOnce(&mut Progress)>(site: &Site, edit: F) {
    let mut held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(progress) = held.as_mut() {
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
fn halted_feeds(site: &Site) -> Vec<bool> {
    let held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.as_ref().map_or_else(Vec::new, |progress| {
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
fn stopping(site: &Site) -> bool {
    let held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.as_ref().is_some_and(|progress| progress.stopping)
}

/// Marks the run finished HOWEVER the task ends.
///
/// # The deadlock this exists to stop
///
/// `finished: None` is the only reading of "a run is in flight", and
/// `/pull/run` refuses to start a second one while it holds. A task that
/// panicked, or that was cancelled at a shutdown, would leave that `None` in
/// place forever and every later press would be refused with `AlreadyRunning`
/// against a run that no longer exists. `Drop` runs on the panic path, so the
/// slot is released on every exit rather than only the happy one.
///
/// It writes only when nothing else has: a run that finished normally has
/// already put its own summary there, and this must not paint over it.
struct Finisher {
    /// Whose slot to release.
    site: Loaded,
}

impl Drop for Finisher {
    fn drop(&mut self) {
        with_progress(&self.site, |progress| {
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
type Checkpoints = std::sync::Arc<[std::sync::atomic::AtomicBool]>;

fn checkpoints_for(groups: &[(String, Vec<Leg>)]) -> Vec<Checkpoints> {
    groups
        .iter()
        .map(|(_, legs)| {
            legs.iter()
                .map(|_| std::sync::atomic::AtomicBool::new(false))
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
    Permanent,
    Credential,
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
        LegOutcome::Credential
    } else {
        LegOutcome::Retry
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
    nth: usize,
    leg: &Leg,
    status: axum::http::StatusCode,
    outcome: LegOutcome,
) {
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
        _ => {
            "its receipt did not read clean. The reason is in the audit journal; \
            this leg remains owed and is eligible for another pass."
        }
    };
    with_progress(site, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
            if feed.last_error.is_none()
                || matches!(outcome, LegOutcome::Credential | LegOutcome::Permanent)
            {
                feed.last_error = Some(format!(
                    "{} answered HTTP {}: {reason}",
                    leg.label,
                    status.as_u16()
                ));
            }
            feed.credential_dead |= outcome == LegOutcome::Credential;
        }
    });
}

/// Run one feed's legs sequentially. Only a response that actually returned
/// advances `legs_done`; dependencies and early breaks remain unattempted.
async fn run_chain<F, Fut>(
    site: Loaded,
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
        if stopping(&site) {
            break;
        }
        if clean.load(std::sync::atomic::Ordering::Relaxed) {
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
        with_progress(&site, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.doing.clone_from(&leg.label);
            }
        });
        attempted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (status, html) = request(Loaded::clone(&site), leg.clone()).await;
        with_progress(&site, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.legs_done = feed.legs_done.saturating_add(1);
            }
        });
        let outcome = leg_outcome(status, &html);
        if matches!(outcome, LegOutcome::Stored | LegOutcome::Empty) {
            clean.store(true, std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        note_leg_failure(&site, nth, leg, status, outcome);
        if leg.route == Route::Spot {
            failed_spot_rank = Some(ladder_rank(&leg.dir));
        }
        if matches!(outcome, LegOutcome::Credential | LegOutcome::Permanent) {
            result = PassOutcome::Halted;
            break;
        }
        result = PassOutcome::Retry;
    }
    with_progress(&site, |progress| {
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
    groups: &[(String, Vec<Leg>)],
    outcomes: &mut [PassOutcome],
    checkpoints: &[Checkpoints],
    request: &F,
) where
    F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
    Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
{
    if stopping(site) {
        return;
    }
    let halted = halted_feeds(site);
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
                leg.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let again = *prior == PassOutcome::Retry;
        with_progress(site, |progress| {
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
                    nth,
                    attempted.load(std::sync::atomic::Ordering::Relaxed),
                    &dead,
                );
                PassOutcome::Retry
            }
        };
        if again && attempted.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            retrying = true;
            with_progress(site, |progress| {
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
        with_progress(site, |progress| {
            progress.retries = progress.retries.saturating_add(1);
        });
    }
}

/// Server-owned passes outlive the browser, but are not persisted across a
/// server restart. Feeds run in parallel; each feed's legs run sequentially.
/// Fixed refusals halt that feed. Transient failures retry up to [`MAX_PASSES`].
/// Idle termination is an observation about store growth, never full coverage.
pub async fn conduct(site: Loaded, legs: Vec<Leg>) {
    conduct_with(site, legs, request_leg).await;
}

/// The request seam permits deterministic coordinator tests without vendors,
/// credentials, alternate production behavior, or a second pass loop.
async fn conduct_with<F, Fut>(site: Loaded, legs: Vec<Leg>, request: F)
where
    F: Fn(Loaded, Leg) -> Fut + Clone + Send + 'static,
    Fut: core::future::Future<Output = (axum::http::StatusCode, String)> + Send + 'static,
{
    let _finisher = Finisher {
        site: Loaded::clone(&site),
    };
    let groups = by_feed(legs);
    let started_rows = rows_now(&site);
    with_progress(&site, |progress| {
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
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(ROWS_TICK).await;
                let seen = rows_now(&site);
                with_progress(&site, |progress| progress.rows_now = seen);
            }
        })
    };
    let mut outcomes = vec![PassOutcome::Clean; groups.len()];
    let checkpoints = checkpoints_for(&groups);
    let mut passes = 0_u32;
    let mut clean_empty = 0_u32;
    while passes < MAX_PASSES {
        if stopping(&site) {
            break;
        }
        let repairing = outcomes.contains(&PassOutcome::Retry);
        let before = rows_now(&site);
        run_pass(&site, &groups, &mut outcomes, &checkpoints, &request).await;
        passes = passes.saturating_add(1);
        let after = rows_now(&site);
        with_progress(&site, |progress| {
            progress.passes = passes;
            progress.rows_now = after;
        });
        if stopping(&site)
            || outcomes
                .iter()
                .all(|outcome| *outcome == PassOutcome::Halted)
        {
            break;
        }
        if after > before {
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
    ticker.abort();
    let current_rows = rows_now(&site);
    with_progress(&site, |progress| {
        progress.rows_now = current_rows;
        progress.finished = Some(run_summary(
            progress,
            &outcomes,
            current_rows.saturating_sub(started_rows),
        ));
    });
}

/// Terminal feeds remain part of the final verdict even while other feeds
/// reach an idle stop. Neither a skipped feed nor a stopped task is success.
fn run_summary(progress: &Progress, outcomes: &[PassOutcome], landed: u64) -> String {
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
            "INCOMPLETE after {} pass(es); {landed} bar(s) added to the store census. \
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
    summary_of(progress.passes, progress.retries, landed, progress.stopping)
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
fn note_dead_chain(site: &Site, nth: usize, attempted: usize, dead: &tokio::task::JoinError) {
    with_progress(site, |progress| {
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

/// Describe an operator stop, pass ceiling, or idle stop. Store growth and
/// receipt verdicts cannot establish coverage of every requested instrument.
#[must_use]
pub fn summary_of(passes: u32, retries: u32, landed: u64, stopped: bool) -> String {
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

    /// One `leg` field, encoded exactly as the page encodes it.
    fn field(route: &str, vendor: &str, dir: &str, label: &str, body: &str) -> String {
        let joined = format!("{route}|{vendor}|{dir}|{label}|{}", enc(body));
        format!("leg={}", enc(&joined))
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
        let legs = legs_from(&field("/pull/spot", "dhan", "1day", "Spot · 1 day", body))
            .expect("one well-formed leg");
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].body, body, "the body reaches the route unchanged");
        assert_eq!(legs[0].route, Route::Spot);
        assert_eq!(legs[0].vendor, "dhan");
        assert_eq!(legs[0].dir, "1day");
        assert_eq!(legs[0].label, "Spot · 1 day");
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

    /// Every field the page reads is in the document, with the name it reads.
    ///
    /// Hand-written JSON has no compiler checking the key names against the
    /// reader, so this is that check.
    #[test]
    fn the_document_carries_every_field_the_page_reads() {
        let progress = Progress {
            passes: 7,
            retries: 2,
            rows_at_start: 10,
            rows_now: 99,
            stopping: true,
            finished: None,
            started: true,
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
            }],
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
        let finished = summary_of(3, 0, 500, false);
        let ceiling = summary_of(MAX_PASSES, 4, 500, false);
        let stopped = summary_of(9, 1, 500, true);

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
    fn claim(site: &Site) {
        let mut held = site
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *held = Some(Progress::claimed());
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
        with_progress(&site, |progress| progress.stopping = true);

        conduct(
            Loaded::clone(&site),
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
        with_progress(&site, |progress| {
            progress.finished = Some("done".to_owned());
        });
        assert!(
            !observed(&site).running(),
            "the summary releases the standoff"
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
        with_progress(&held, |progress| {
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
            halted_feeds(&held),
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
            halted_feeds(&fresh).is_empty(),
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
            conduct_with(Loaded::clone(&held), legs, request),
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
            (1, 0, 0)
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
            (3, 0, 0)
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
                with_progress(&site, |progress| progress.stopping = true);
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
        with_progress(&held, |progress| {
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
            let summary = run_summary(&progress, &passes.outcomes, 0);
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
                with_progress(&site, |p| p.stopping = true);
                std::future::ready(receipt(200, "STORED"))
            })
            .await;
        let progress = observed(&passes.site);
        let feed = &progress.feeds[0];
        assert_eq!((feed.legs_done, feed.skipped, feed.retries), (1, 2, 1));
        assert!(feed.finished && feed.doing.is_empty());
        assert!(run_summary(&progress, &passes.outcomes, 0).contains("pressed stop"));
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
            [true, true, false]
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
                with_progress(&site, |p| p.stopping = true);
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
            (5, 1, 0, 1)
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
            assert_eq!(rows_now(&passes.site), 0);
        }
        let progress = observed(&passes.site);
        assert!(
            progress.feeds[0]
                .last_error
                .as_ref()
                .unwrap()
                .contains("HTTP 200")
        );
        let summary = summary_of(MAX_PASSES, progress.retries, 0, false);
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
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, _leg| std::future::ready(receipt(503, "NOT STARTED")),
        )
        .await;
        run_pass(
            &held,
            &groups,
            &mut outcomes,
            &checkpoints,
            &|_site, _leg| std::future::ready(receipt(422, "NOT STARTED")),
        )
        .await;
        run_pass(
            &held,
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
        assert!(run_summary(&progress, &outcomes, 0).contains("INCOMPLETE"));
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
            rows_at_start: 0,
            rows_now: 500,
            stopping: false,
            finished: None,
            started: true,
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
                },
            ],
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
        with_progress(&site, |progress| {
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
                [false, false]
                    .into_iter()
                    .map(std::sync::atomic::AtomicBool::new)
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
