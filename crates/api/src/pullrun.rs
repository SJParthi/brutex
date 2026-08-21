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
    /// Whether this feed's CREDENTIAL is dead, so re-asking cannot help.
    ///
    /// # Why this is a field and not another `last_error`
    ///
    /// Because it changes what the loop DOES, not only what the page says. Every
    /// other failure on this path is transient by default and retried without a
    /// ceiling — the operator's rule of 2026-08-20, and the right default for a
    /// dropped socket or a throttle. A dead token is the one reason that is
    /// certain rather than probable: §8 forbids minting one here, so the value
    /// cannot change until somebody refreshes it outside this process, and every
    /// pass until then spends requests to be refused identically.
    ///
    /// Set once, never cleared for the life of the run. It is per FEED, because
    /// a credential is — the same reason the seats and the governors are.
    pub credential_dead: bool,
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

            // IS THIS THE CREDENTIAL? ASKED HERE, BECAUSE NOTHING ON THIS PATH
            // EVER ASKED IT.
            //
            // `autopilot::classify` has existed and been tested since the
            // autopilot was written, and it had TWO call sites, both inside
            // `autopilot.rs`. The manual run — the one an operator presses —
            // had no credential handling of any kind. A dead token therefore
            // produced `retries` climbing and a `lastError` reading "answered
            // HTTP 401 … the leg is owed and will be asked for again", for up
            // to `MAX_PASSES` × `RETRY_WAIT` — **two hours of 401s against a
            // token another system shares**, with the word "credential"
            // appearing nowhere.
            //
            // §8 is explicit that this repository never mints one, so there is
            // nothing to retry INTO: the refreshed value is read on the next
            // pull, and every request spent before then is spent to be told the
            // same thing. `CLAUDE.md` §4 — degrade loudly and name the reason.
            //
            // PER FEED, NEVER THE WHOLE RUN. A credential is per vendor, so a
            // dead Dhan token must not stop Groww — the same reason the seats
            // and the governors are per feed. `conduct` skips only the feed
            // this fires on.
            let dead_credential =
                crate::autopilot::classify(&html) == crate::autopilot::Trouble::Credential;

            let why = if dead_credential {
                format!(
                    "{} answered HTTP {} and the reason is the CREDENTIAL, not \
                     the network. This feed is halted for the rest of the run: \
                     §8 forbids minting a token here, so re-asking cannot fix \
                     it and every attempt would spend a request to be refused \
                     identically. Refresh the token where it is minted; the new \
                     value is read on the next pull, and the run resumes from \
                     what the store already holds. Other feeds are unaffected.",
                    leg.label,
                    status.as_u16()
                )
            } else {
                format!(
                    "{} answered HTTP {} and its receipt did not read good. The \
                     reason is in the audit journal; this line only records that \
                     the leg is owed and will be asked for again.",
                    leg.label,
                    status.as_u16()
                )
            };
            with_progress(&site, |progress| {
                if let Some(feed) = progress.feeds.get_mut(nth) {
                    // THE FIRST REASON, KEPT — not the last. A later failure
                    // must not paint over the one that started the trouble.
                    //
                    // A CREDENTIAL DEATH IS THE ONE EXCEPTION, and it has to
                    // be: it arrives on whichever leg happens to run after the
                    // token expires, so an earlier transport blip would
                    // otherwise hide the only reason that cannot be waited out.
                    if feed.last_error.is_none() || dead_credential {
                        feed.last_error = Some(why);
                    }
                    if dead_credential {
                        feed.credential_dead = true;
                    }
                }
            });
            if dead_credential {
                // THE REST OF THIS FEED'S LEGS ARE NOT ATTEMPTED. They would
                // each earn the same 401 against the same dead token.
                break;
            }
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
        // A FEED WHOSE CREDENTIAL DIED IS NOT RE-OPENED, and its `finished`
        // stays true so the page keeps showing it halted rather than pending.
        let halted = halted_feeds(&site);
        with_progress(&site, |progress| {
            for feed in &mut progress.feeds {
                if feed.credential_dead {
                    continue;
                }
                feed.finished = false;
                feed.legs_done = 0;
            }
        });
        let mut flying = Vec::with_capacity(groups.len());
        for (nth, (_vendor, group)) in groups.iter().enumerate() {
            // SKIPPED, NOT RE-ASKED. §8 forbids minting a token here, so the
            // value cannot change until somebody refreshes it outside this
            // process — every pass until then would spend this feed's whole
            // ladder to be refused identically. The other feeds are untouched,
            // because a credential is per vendor.
            if halted.get(nth).copied().unwrap_or(false) {
                continue;
            }
            // THE FEED INDEX TRAVELS WITH THE HANDLE. `flying` is now SHORTER
            // than `groups` whenever a feed is halted, so the position in this
            // vector is no longer the position in `progress.feeds` — and
            // `note_dead_chain` writes by that index. Pairing them is what stops
            // a panic being reported against somebody else's feed.
            flying.push((
                nth,
                tokio::spawn(run_chain(Loaded::clone(&site), nth, group.clone())),
            ));
        }
        let mut failed = false;
        for (nth, chain) in flying {
            match chain.await {
                Ok(chain_failed) => failed |= chain_failed,
                // A PANICKED OR CANCELLED CHAIN IS A FAILED CHAIN — AND IT IS
                // NAMED ON THE FEED IT KILLED.
                //
                // This read `chain.await.unwrap_or(true)`, which counted a dead
                // chain as failed and said nothing about WHICH feed died. The
                // fingerprint of that, measured 2026-08-20: `doing` frozen at
                // the leg it was on, `finished` still false, `lastError` still
                // null, no journal record, no socket — indefinitely, because a
                // task that panics never reaches the code that clears its own
                // fields. A feed that has stopped is then indistinguishable
                // from one that is working slowly, which is the confusion this
                // whole module exists to remove.
                //
                // The panic itself goes to STDERR through the panic hook and
                // never reaches `telemetry`, so this line is the only surface
                // that can say it happened at all.
                Err(dead) => {
                    failed = true;
                    note_dead_chain(&site, nth, &dead);
                }
            }
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
fn note_dead_chain(site: &Site, nth: usize, dead: &tokio::task::JoinError) {
    with_progress(site, |progress| {
        if let Some(feed) = progress.feeds.get_mut(nth) {
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
                credential_dead: false,
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
        f.write_all(
            b"Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest\n\
              NIFTY.NFO,01/07/2025,09:16:16,27674,0,0,0,0,65,65\n\
              NIFTY.NFO,01/07/2025,09:17:04,27680,0,0,0,0,40,70\n",
        )
        .expect("write");

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
        // `conduct`'s pass loop is deliberately unbounded on failure: a pass
        // whose legs failed sleeps `RETRY_WAIT` and goes again, up to
        // `MAX_PASSES`. That is right for an operator waiting out a vendor and
        // catastrophic for a test — MEASURED, by breaking this test's own
        // fixture: with no data rows the leg fails, the loop retries, and the
        // test ran past sixty seconds instead of failing. Unbounded it would
        // have taken `MAX_PASSES` x `RETRY_WAIT` ~= two hours and WEDGED CI
        // rather than reddening it.
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
                vec![Leg {
                    route: Route::Spot,
                    vendor: pull::vendor::Feed::TrueData.wire().to_owned(),
                    dir: "1min".to_owned(),
                    label: "archive · 1 minute".to_owned(),
                    body: format!(
                        "target=swept&vendor={}&from=2025-07-01&to=2025-07-01&folder={}",
                        pull::vendor::Feed::TrueData.wire(),
                        folder.display()
                    ),
                }],
            ),
        )
        .await;
        assert!(
            ran.is_ok(),
            "the press did not finish inside {bound:?}. On this path that means \
             the leg FAILED and the pass loop is retrying it — the loop is \
             unbounded on failure by design, so the run would continue for \
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

    /// **THE SPAWN LOOP CONSULTS THE SKIP LIST, AND PAIRS THE INDEX WITH THE
    /// HANDLE.**
    ///
    /// Two properties that fail independently, and the second is the subtle one.
    ///
    /// Skipping makes `flying` SHORTER than `groups`, so a position in that
    /// vector is no longer a position in `progress.feeds` — and `note_dead_chain`
    /// writes by that index. Without the pairing, a panic in one feed would be
    /// reported against another feed's row, and only when a credential died AND
    /// something panicked in the same pass. That is a bug nobody would find by
    /// reading.
    ///
    /// Source-text because the alternative is a twenty-second sleep per pass:
    /// the loop is only observable through a run, and a run whose legs fail is
    /// deliberately unbounded. The skip's INPUT is driven for real by the test
    /// above.
    #[test]
    fn the_spawn_loop_skips_halted_feeds_and_carries_the_index_with_the_handle() {
        let source = include_str!("pullrun.rs");
        let body = source
            .split_once("pub async fn conduct")
            .expect("conduct exists")
            .1;
        let body = &body[..body
            .find("\n}\n")
            .expect("conduct's body ends at a column-0 brace")];

        assert!(
            body.contains("let halted = halted_feeds(&site);"),
            "the pass loop must read the skip list, or a dead token keeps \
             costing this feed's whole ladder every pass"
        );
        assert!(
            body.contains("flying.push((\n                nth,"),
            "the feed index must travel WITH the handle: `flying` is shorter \
             than `groups` whenever a feed is skipped, and `note_dead_chain` \
             writes by that index"
        );
        // THE OLD SHAPE, ASSEMBLED AT RUN TIME so this assertion does not match
        // its own source — three times now a source-text test in this workspace
        // has done exactly that.
        let bare = format!(
            "{}{}",
            "for (nth, chain) in flying.into_iter()", ".enumerate()"
        );
        assert!(
            !body.contains(bare.as_str()),
            "enumerating `flying` re-derives the index from a vector that no \
             longer matches `progress.feeds`"
        );
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
}
