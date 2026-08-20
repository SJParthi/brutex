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
//! rewrites it in place; nothing accumulates per pass.

use crate::census;
use crate::server::{Loaded, Site, percent_decode};

/// How many passes one press may make.
///
/// A window can be larger than one sitting at a legal rate, so the run keeps
/// going. This is not a guess at how many passes are needed -- it is a stop for
/// a loop that would otherwise be unbounded if the vendor never recovered, and
/// it is REPORTED when it is reached rather than being silently the end.
pub const MAX_PASSES: u32 = 400;

/// How many clean empty passes mean "there is nothing left to get".
///
/// **Three, and only for a pass that FAILED AT NOTHING.** Operator's rule of
/// 2026-08-20: a vendor answering "no data available" is acceptable after three
/// attempts; every other outcome is retried until it succeeds.
///
/// | Pass gained no bars and… | Meaning | What the loop does |
/// |---|---|---|
/// | nothing refused, nothing errored | there is no more data | count toward three, then stop |
/// | a leg failed, refused or dropped | rate, socket, power, credential | **retry, with no limit** |
///
/// The two are indistinguishable by bar count alone -- a throttled pass and a
/// finished window both store zero -- which is why the cause is what decides.
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
    /// How many it has finished this pass.
    pub legs_done: u32,
    /// The label of the leg on the wire right now.
    pub doing: String,
    /// How many times a failure sent this chain round again.
    pub retries: u32,
    /// The first reason this chain failed, kept rather than the last, so the
    /// page names a CAUSE rather than whatever went wrong most recently.
    pub last_error: Option<String>,
    /// Whether this chain has nothing left to do.
    pub finished: bool,
}

/// The whole run, as the page reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// How many passes have completed.
    pub passes: u32,
    /// How many passes were retried after a failure.
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
fn rows_now(site: &Site) -> u64 {
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
                     press Pull again and it resumes from what is stored."
                        .to_owned(),
                );
            }
        });
    }
}

/// Runs one vendor's legs, in order, and answers whether any of them failed.
///
/// # Sequential, and not as a preference
///
/// A rate budget is per vendor, so two legs fired at one broker together spend
/// one ceiling twice. `pull::fold`'s ladder is the other half: the day pass for
/// a month must land before the minute pass for it, and spot before the
/// derivatives that reference it. [`by_feed`] has already sorted them; this
/// awaits them in that order.
///
/// # How a leg is judged
///
/// `status.is_success()` AND the receipt's own verdict element reads good. The
/// status alone is not enough — a refusal inside a run that reached the vendor
/// and stored nothing is answered 200 with a receipt that says so, which is
/// exactly the case the browser loop used to read through `readReceipt`. The
/// `badge good` marker is the same one `crates/api` tests already assert on.
///
/// What is recorded is the leg and its status, NOT the receipt's prose: the
/// detail is already in the journal, and copying an HTML fragment into a status
/// document would be a second, drifting copy of it.
async fn run_chain(site: Loaded, nth: usize, legs: Vec<Leg>) -> bool {
    let mut failed = false;
    for (index, leg) in legs.iter().enumerate() {
        if stopping(&site) {
            break;
        }
        let label = leg.label.clone();
        with_progress(&site, |progress| {
            if let Some(feed) = progress.feeds.get_mut(nth) {
                feed.doing = label;
                feed.legs_done = u32::try_from(index).unwrap_or(u32::MAX);
            }
        });

        let (status, html) = match leg.route {
            Route::Spot => {
                let (status, _receipt, body) = crate::server::pull_spot(
                    axum::extract::State(Loaded::clone(&site)),
                    leg.body.clone(),
                )
                .await;
                (status, body.0)
            }
            Route::Fno => {
                let (status, body) = crate::server::pull_fno(
                    axum::extract::State(Loaded::clone(&site)),
                    leg.body.clone(),
                )
                .await;
                (status, body.0)
            }
        };

        if !(status.is_success() && html.contains("badge good")) {
            failed = true;
            let why = format!(
                "{} answered HTTP {} and its receipt did not read good. The reason \
                 is in the audit journal; this line only records that the leg is \
                 owed and will be asked for again.",
                leg.label,
                status.as_u16()
            );
            with_progress(&site, |progress| {
                if let Some(feed) = progress.feeds.get_mut(nth) {
                    // THE FIRST REASON, KEPT — not the last. A later failure
                    // must not paint over the one that started the trouble.
                    if feed.last_error.is_none() {
                        feed.last_error = Some(why);
                    }
                }
            });
        }
    }

    let done = u32::try_from(legs.len()).unwrap_or(u32::MAX);
    with_progress(&site, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
            feed.legs_done = done;
            feed.doing = String::new();
            feed.finished = true;
        }
    });
    failed
}

/// The whole run: passes over every feed until the window is satisfied.
///
/// # What ends it, and what does not
///
/// | Pass gained bars | A leg failed | What happens |
/// |---|---|---|
/// | yes | either | another pass, immediately |
/// | no | yes | another pass after [`RETRY_WAIT`], **with no limit** |
/// | no | no | one of [`CLEAN_EMPTY_PASSES`], then finish |
///
/// The operator's rule of 2026-08-20 in full: *"even after 3 attempts … where
/// it provides the result as no data available means then it is entirely
/// acceptable … other than this issue it can be any kinds of extreme worst case
/// scenarios like power cut or some other issue or internet issue or duplicate
/// issue or uniqueness issues … it should always be fetched until it is
/// successful, it should never ever stop"*. A failed pass is transient BY
/// DEFAULT and is retried without a ceiling; only a pass that asked for
/// everything and was refused nothing may count toward the end.
///
/// # Why a panicking chain is a failed chain
///
/// `JoinHandle` answers `Err` for a panicked or cancelled task. Treating that
/// as "this chain is done" would end a run on the one outcome that most needs
/// retrying, so it is folded into `failed` alongside an ordinary refusal.
///
/// # Cost
///
/// Per pass: one census read before, one after, and one spawn per vendor. The
/// per-leg path adds a mutex take and a `String` clone for the status document
/// — O(1) each, and no allocation that grows with how many passes came before.
pub async fn conduct(site: Loaded, legs: Vec<Leg>) {
    // RELEASED ON EVERY EXIT, INCLUDING A PANIC. See `Finisher`.
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

    // THE STORE IS READ WHILE THE PASS RUNS, NOT ONLY BETWEEN PASSES.
    //
    // `rows_now` was refreshed at pass boundaries alone. A pass is one leg per
    // feed and a single F&O leg can run for half an hour, so for that whole
    // time the document said `rowsNow: 0` -- measured 2026-08-20 20:41, with
    // **2,213 bar files on disk** and the run reporting zero. The operator read
    // it exactly as it was written: *"keeps on running but no results stored
    // anywhere"*.
    //
    // A ticker rather than a read per leg, because the leg is the thing that is
    // slow: updating after each one would still leave the half-hour gap it is
    // meant to fill. One manifest read every [`ROWS_TICK`] costs far less than
    // the page's own two-second poll of the document it feeds.
    //
    // ABORTED, NEVER LEAKED. The handle is dropped at the end of `conduct`
    // after an explicit `abort`, so a finished run leaves nothing reading the
    // store behind it.
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

    let mut passes = 0_u32;
    let mut retries = 0_u32;
    let mut clean_empty = 0_u32;

    while passes < MAX_PASSES {
        if stopping(&site) {
            break;
        }
        let before = rows_now(&site);

        // PARALLEL ACROSS FEEDS. One task per vendor, all started before any is
        // awaited — awaiting each in turn as it is spawned would serialise them
        // and quietly undo the whole point.
        with_progress(&site, |progress| {
            for feed in &mut progress.feeds {
                feed.finished = false;
                feed.legs_done = 0;
            }
        });
        let mut flying = Vec::with_capacity(groups.len());
        for (nth, (_vendor, group)) in groups.iter().enumerate() {
            flying.push(tokio::spawn(run_chain(
                Loaded::clone(&site),
                nth,
                group.clone(),
            )));
        }
        let mut failed = false;
        for chain in flying {
            // A PANICKED OR CANCELLED CHAIN IS A FAILED CHAIN.
            failed |= chain.await.unwrap_or(true);
        }

        passes = passes.saturating_add(1);
        let after = rows_now(&site);
        with_progress(&site, |progress| {
            progress.passes = passes;
            progress.rows_now = after;
        });

        if stopping(&site) {
            break;
        }
        if after > before {
            // PROGRESS. Whatever else went wrong, bars landed, so the window is
            // not finished and the next pass is worth making.
            clean_empty = 0;
            continue;
        }
        if failed {
            clean_empty = 0;
            retries = retries.saturating_add(1);
            with_progress(&site, |progress| progress.retries = retries);
            // WAITED, NOT SPUN. The governor backs off inside a pass; this is
            // the gap between passes, which nothing else covers.
            tokio::time::sleep(RETRY_WAIT).await;
            continue;
        }
        clean_empty = clean_empty.saturating_add(1);
        if clean_empty >= CLEAN_EMPTY_PASSES {
            break;
        }
    }

    ticker.abort();
    let landed = rows_now(&site).saturating_sub(started_rows);
    let summary = summary_of(passes, retries, landed, stopping(&site));
    with_progress(&site, |progress| progress.finished = Some(summary));
}

/// What the run did, in a sentence that distinguishes the three ways it can end.
///
/// Separated from [`conduct`] so it can be asserted directly: the difference
/// between "the window is finished" and "this is a runaway stop" is the whole
/// value of the summary, and a test that had to run four hundred passes to
/// check it would not be run.
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
            "Stopped after {passes} pass(es); {landed} bar(s) landed before you \
             pressed stop.{note}"
        );
    }
    if passes >= MAX_PASSES {
        return format!(
            "Reached the {MAX_PASSES}-pass ceiling with {landed} bar(s) landed.{note} \
             This is a runaway stop and NOT a verdict on the window — press Pull \
             again to continue from where this stopped; nothing already stored is \
             refetched."
        );
    }
    format!(
        "{passes} pass(es), {landed} bar(s) landed.{note} The last \
         {CLEAN_EMPTY_PASSES} asked for everything, were refused nothing and gained \
         nothing — which is what \"no more data\" looks like, and the only reading \
         this run treats as finished."
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

        assert!(finished.contains("no more data"), "{finished}");
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
}
