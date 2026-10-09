//! `/gaps.json` -- whether one stored series is complete across a range of
//! months, judged against a calendar its peers vote for.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach, and
//! the path an intra-doc link now needs to name a `server.rs` item that is no
//! longer in scope here. No route answers differently because of the move.

use super::{
    Addressed, CensusNow, CensusStamps, Loaded, Site, audit_cash_schedule_window,
    calendar_admission_refused, census_holds, census_now_stamped, month_param, param,
    spot_months_by_identity,
};
use crate::{bars, census, ingest, render};
use pull::session::Day;

/// Whether one stored instrument-month is COMPLETE, and where it is not.
///
/// # `pull::gaps` had zero callers, and that is the hole this fills
///
/// The module classifies every absent minute into four reasons — `closed`,
/// `outside-window`, `vendor-hole`, `unmeasured` — with a taxonomy built
/// precisely so a complete series stops reading as a short one. Nothing called
/// it. Measured across the whole workspace: `grep -rn "gaps::"` outside its own
/// file returned **nothing**, so every fact it can state was unreachable and
/// the store could not be asked the one question that matters after a pull —
/// *is this month whole?*
///
/// P-68 named this as its own unclosed half: an empty chunk is now loud at the
/// moment it happens, and the STATE afterwards was still undetectable. A
/// backfill that lost a chunk in March 2021 leaves a store that looks exactly
/// like one that did not, because `n_valid` counts what is there and nothing
/// counts what should be.
///
/// # Why a count of bars can never answer it
///
/// The arithmetic answer — "375 bars a day times the trading days" — is wrong
/// in both directions and was the reason this was hard. Muhurat trades one hour
/// in the afternoon; a disaster-recovery Saturday has a two-hour hole in the
/// middle by design; five Diwali sessions before 2025 have a length this build
/// does not know. Counting calls all three a loss. `classify` consults the
/// calendar per day and only `vendor-hole` is a loss.
///
/// # Cost, stated rather than claimed
///
/// One pass over the month's bars and one over its minutes: ~44,640 minutes for
/// a 31-day month against at most ~11,625 stored one-minute bars, both walked
/// by a single forward cursor that never rewinds. That is **not O(1)** and is
/// not claimed to be — `CLAUDE.md` §3 rule 4 names the five operations that
/// must be constant (bar lookup, condition lookup, mask evaluation, duplicate
/// rejection, result append) and a completeness audit is none of them. It is
/// linear in the month, bounded by a constant month, and asked once per
/// instrument-month by an operator rather than in any loop. `MAX_GAPS` bounds
/// the answer's SIZE, and `Ledger::truncated` says so out loud when it bites —
/// a truncated ledger that read like a complete one would report a store
/// healthier than it is.
pub(super) async fn gaps_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("");
    let json = || {
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )]
    };
    let refuse = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        )
    };

    let mut asked = match Addressed::parse(query) {
        Ok(asked) => asked,
        Err(why) => return refuse(why),
    };
    // AN ADDRESS THE STORE REFUSES IS A 400, NOT AN ABSENT MONTH (probeapi-2,
    // D-1200). `audit_one` folds every `open` refusal into `absent_file`, so an
    // empty or `..` segment used to answer 200 with every expected minute
    // classified `vendor-hole` — a missing-data report about a path that can
    // never exist — while `/bars.json` refused the same query. The segments do
    // not depend on the month, so one check covers every month in the range.
    if let Err(why) = asked.store_path_is_valid() {
        return refuse(why);
    }
    // `to` IS OPTIONAL AND DEFAULTS TO `month`, so the single-month question
    // stays a single-month question and the range is opt-in. An unparseable
    // `to` REFUSES rather than collapsing to one month: a range that silently
    // becomes narrower answers a different question than the one asked, and
    // reports the months it never looked at as absent from the answer.
    let last_month = if param(query, "to").is_empty() {
        asked.month
    } else {
        match month_param(query, "to") {
            Ok(month) => month,
            Err(why) => return refuse(why),
        }
    };
    if last_month < asked.month {
        return refuse(format!(
            "the range runs backwards: {} is before {}",
            last_month, asked.month
        ));
    }

    // THE SPAN, WALKED ONCE, AND THE CALENDAR DERIVED FROM IT ONCE.
    //
    // Collected before the audit rather than during it, because
    // `calendar_of::derive` reads the whole span's daily rung to decide which
    // days traded — asking it per month would re-derive the same answer once
    // per month and, worse, would give each month a calendar built from its own
    // data alone.
    let mut span = Vec::new();
    let mut cursor = asked.month;
    let mut truncated_range = false;
    loop {
        if span.len() >= MAX_AUDIT_MONTHS {
            // NEVER A SILENT `take(N)`. A ceiling that stopped quietly would
            // report the months it never reached as no news, which is the
            // direction that reads as health.
            truncated_range = true;
            break;
        }
        span.push(cursor);
        if cursor >= last_month {
            break;
        }
        let Some(next) = next_month(cursor) else {
            break;
        };
        cursor = next;
    }

    // THE CALENDAR MUST NOT COME FROM THE SERIES BEING AUDITED, AND THE FIRST
    // DRAFT OF THIS ROUTE TOOK IT FROM EXACTLY THERE.
    //
    // The problem being solved is real: `pull::calendar`'s typed table ends at
    // `LAST_DAY = 2026-08-21`, and on 2026-08-29 this route answered
    // `expected 5,625, held 7,500, lost 0` for NIFTY at one minute in 2026-08 —
    // twenty trading days of bars against fifteen the table could vouch for.
    // Every day past the table falls to `Unmeasured`.
    //
    // The obvious repair was `calendar_of::cached` for THIS series, which never
    // goes stale. **It is also completely circular, and a test proved it in one
    // run.** `calendar_of` derives each session's LENGTH from the first and
    // last minute bar and its WINDOWS from contiguous runs of them, so a
    // missing minute does not read as a hole — it reads as the edge of a
    // window. Measured: with one minute deleted mid-session, the answer was
    // `expected 8,249, held 8,249, lost 0` and the gap list carried
    // `{"day":19738,"from":556,"to":556,"reason":"outside-window"}`. The hole
    // became a fact about the exchange.
    //
    // `calendar_of::agree`'s own doc names this: *"A shorter reading is that
    // instrument's hole, not a shorter exchange day — the opposite rule would
    // let one vendor's missing morning shorten the calendar for everything
    // else, which is how an expected-bar count becomes quietly too small and a
    // real loss stops being reported."*
    //
    // So the readings are the PEERS — every other feed's copy of a spot series
    // on this exchange and segment — agreed by union, and the audited series is
    // excluded from its own denominator. A bar is proof the exchange traded;
    // silence is not proof that it did not.
    if asked.timeframe != store::path::Timeframe::MINUTE_1 {
        return refuse("minute coverage requires timeframe=1min; coarser bars cannot prove their source minutes".to_owned());
    }
    let peers = if asked.exchange == "NSE" && asked.segment == "CASH" && asked.contract.is_none() {
        // A peer's auction eligibility is not this stock's eligibility. No
        // peer is consulted, so none can be named as unreadable either.
        PeerCalendar {
            calendar: None,
            from: Vec::new(),
            unreadable: Vec::new(),
        }
    } else {
        // OFF THE ASYNC WORKERS, AND ADMITTED: the peer vote derives one
        // calendar per peer series on a miss. W1-api2-11, D-1443.
        let peers_site = std::sync::Arc::clone(&site);
        match crate::detail::run_calendar(move || {
            let peers = peer_calendar(&peers_site, &asked);
            (asked, peers)
        })
        .await
        {
            Ok((back, peers)) => {
                asked = back;
                peers
            }
            Err(why) => return calendar_admission_refused(&why),
        }
    };
    let (months, peers) = match audit_span(&site, asked, span.clone(), peers).await {
        Ok(done) => done,
        Err(why) => return calendar_admission_refused(&why),
    };

    (
        axum::http::StatusCode::OK,
        json(),
        gaps_body(&months, &span, &peers, truncated_range),
    )
}

/// Every month of a `/gaps.json` span, audited OFF THE ASYNC WORKERS.
///
/// Each month's bar file and its dated cash-session evidence are read in the
/// calendar pool the peer vote already used, in one admission for the whole
/// span. The evidence reader is an `async fn` whose local variant never awaits
/// a fetch, but it reads and locks files inline, so it is driven to completion
/// on the blocking thread by the runtime's handle rather than on a worker.
/// `peers` goes in and comes back, because the answer renders from it.
/// Split from `gaps_json` for the workspace's 100-line ceiling. W1-api2-11,
/// D-1508.
///
/// # Errors
///
/// [`crate::detail::RunError`] when the calendar pool refuses or cannot join.
async fn audit_span(
    site: &Loaded,
    asked: Addressed,
    span: Vec<store::path::YearMonth>,
    peers: PeerCalendar,
) -> Result<(Vec<AuditedMonth>, PeerCalendar), crate::detail::RunError> {
    let runtime = tokio::runtime::Handle::current();
    let audit_site = std::sync::Arc::clone(site);
    crate::detail::run_calendar(move || {
        let mut months = Vec::with_capacity(span.len());
        for month in &span {
            let schedule = runtime.block_on(audit_cash_schedule(&audit_site, &asked, *month));
            let mut audited = audit_one(
                &audit_site,
                &asked,
                *month,
                peers.calendar.as_ref(),
                schedule.as_ref().ok().and_then(Option::as_ref),
            );
            audited.evidence_error = schedule.err();
            months.push(audited);
        }
        (months, peers)
    })
    .await
}

/// The audit's whole answer, from the months it walked.
///
/// Split from the handler so the route stays inside the workspace's 100-line
/// ceiling, and because rendering has nothing to do with deciding: everything
/// here is a fold over `months` or a fact about the calendar that produced
/// them, and none of it opens a file.
fn gaps_body(
    months: &[AuditedMonth],
    span: &[store::path::YearMonth],
    peers: &PeerCalendar,
    truncated_range: bool,
) -> String {
    // THE ROLL-UP IS A SUM OF THE MONTHS, not a second computation over them. A
    // total that could disagree with the rows below it is a total nobody can
    // act on.
    let expected: u64 = months.iter().map(|m| u64::from(m.expected)).sum();
    let held: u64 = months.iter().map(|m| u64::from(m.held)).sum();
    let lost: u64 = months.iter().map(|m| u64::from(m.lost)).sum();
    let absent_files = months.iter().filter(|m| m.absent_file.is_some()).count();
    let rows = months
        .iter()
        .map(AuditedMonth::render)
        .collect::<Vec<_>>()
        .join(",");

    let unmeasured: u64 = months.iter().map(|m| u64::from(m.unmeasured)).sum();

    // THE CALENDAR THAT DECIDED THESE NUMBERS, NAMED IN THE ANSWER.
    //
    // `expected` counts only days the calendar can vouch for. A day it cannot
    // is `Unmeasured`: it adds nothing to `expected`, it is not a loss, and no
    // claim is made either way — which is right, and which makes `held` LARGER
    // than `expected` for any span running past the calendar's edge.
    //
    // Read without this block that pair is an arithmetic bug, and an operator
    // right to distrust it has no way to find out that it is not one. Measured
    // before the derived calendar was wired in: NIFTY at one minute for 2026-08
    // answered `expected 5,625, held 7,500, lost 0` — twenty trading days of
    // bars against fifteen the TABLE could vouch for, because
    // `pull::calendar::LAST_DAY` is 2026-08-21 and the request was made on
    // 2026-08-29.
    //
    // `covers_span` is the honest statement, and it is source-independent:
    // whichever calendar answered, what an operator needs to know is whether
    // the span they ASKED for is inside what can be vouched for.
    //
    // The bounds are the ones actually used. With peers, that is the union of
    // their spans; without, the typed table's `FIRST_DAY..=LAST_DAY`, which is
    // where `stale` still means something and is reported.
    let (first_bound, last_bound, source) = peers.calendar.as_ref().map_or_else(
        || (pull::calendar::FIRST_DAY, pull::calendar::LAST_DAY, "table"),
        |cal| (cal.first_day(), cal.last_day(), "peers"),
    );
    let first_known =
        pull::session::Day::from_days(u32::try_from(first_bound.max(0)).unwrap_or(u32::MAX));
    let last_known =
        pull::session::Day::from_days(u32::try_from(last_bound.max(0)).unwrap_or(u32::MAX));
    let day_text = |day: Result<pull::session::Day, _>| {
        day.map_or_else(
            |_| "null".to_owned(),
            |d| format!(r#""{:04}-{:02}-{:02}""#, d.year(), d.month(), d.day()),
        )
    };
    // THE TABLE'S STALENESS IS STILL REPORTED, because it is still the fallback
    // and it is still a maintenance fact. It is `false` when peers answered:
    // the flag is about whichever calendar bounded this answer, not about a
    // constant nobody consulted.
    let stale = source == "table"
        && ingest::today_ist()
            .is_ok_and(|today| i64::from(today.days_from_epoch()) > pull::calendar::LAST_DAY);
    // AN ABSENT OR SHORT CALENDAR IS ITS OWN ANSWER, NOT A ZERO. A store with
    // one feed's copy of one symbol has nothing independent to vote, so every
    // minute falls to `Unmeasured` and `expected` is 0 beside a `held` of
    // hundreds of thousands. That is not a store owing nothing; it is a question
    // this build cannot settle, and it says so rather than reporting a clean
    // sheet an operator would read as health.
    let calendar_days = peers
        .calendar
        .as_ref()
        .map_or(0, pull::calendar::Calendar::span);
    let covers_span = span.first().is_some_and(|m| {
        pull::session::Day::new(m.year(), m.month(), 1)
            .is_ok_and(|d| i64::from(d.days_from_epoch()) >= first_bound)
    }) && span.last().is_some_and(|m| {
        pull::session::Day::new(m.year(), m.month(), 1)
            .is_ok_and(|d| i64::from(d.end_of_month().days_from_epoch()) <= last_bound)
    });
    let voted = peers
        .from
        .iter()
        .map(|name| render::json_string(name))
        .collect::<Vec<_>>()
        .join(",");
    // A PEER THAT COULD NOT BE READ IS NAMED, not counted as a peer that held
    // nothing: see `PeerCalendar::unreadable`.
    let unreadable = peers
        .unreadable
        .iter()
        .map(|name| render::json_string(name))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"{{"expected":{expected},"held":{held},"lost_minutes":{lost},"unmeasured_minutes":{unmeasured},"months":{},"months_absent":{absent_files},"truncated":{truncated_range},"calendar":{{"first":{},"last":{},"days":{calendar_days},"source":"{source}","stale":{stale},"covers_span":{covers_span},"voted_by":[{voted}],"unreadable":[{unreadable}]}},"month":[{rows}]}}"#,
        months.len(),
        day_text(first_known),
        day_text(last_known),
    )
}

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
pub(super) struct PeerCalendar {
    /// The agreed calendar, or `None` when nothing independent could vote.
    pub(super) calendar: Option<pull::calendar::Calendar>,
    /// The `feed:symbol` readings that voted, sorted, for the answer to name.
    pub(super) from: Vec<String>,
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
    pub(super) unreadable: Vec<String>,
}

/// Build a [`PeerCalendar`] for the series `asked` addresses.
pub(super) fn peer_calendar(site: &Loaded, asked: &Addressed) -> PeerCalendar {
    peer_calendar_reading(site, asked, census_now_stamped)
}

/// [`peer_calendar`], with the census it derives from passed in.
///
/// Production passes [`census_now_stamped`] and nothing else, for the reason
/// [`calendar_json_reading`](super::calendar_route::calendar_json_reading)
/// gives: a test installs a newer manifest after the census is read and before
/// a peer's calendar is kept. D-0695.
pub(super) fn peer_calendar_reading(
    site: &Site,
    asked: &Addressed,
    census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
) -> PeerCalendar {
    // A FRESH CENSUS, for the reason D-0318 records: a list captured before the
    // store had anything in it reports a complete store as short.
    //
    // FRESH ON THE MANIFESTS' STAMPS, NOT RE-READ PER REQUEST. `census_now`
    // re-reads a manifest whose modified time moved and shares the cached
    // census otherwise, so a pull is still visible on the next audit and a
    // steady-state audit reads no manifest bytes. D-0686. Those stamps key
    // each peer's kept calendar: see `census_now_stamped`.
    let (stamps, (fresh, _)) = census(site);
    let mut readings: Vec<(String, pull::calendar::Calendar)> = Vec::new();
    let mut unreadable: Vec<String> = fresh
        .iter()
        .filter(|census| matches!(census.state, census::Census::Unreadable { .. }))
        .map(|census| census.vendor.as_str().to_owned())
        .collect();
    for vendor_census in fresh.as_slice() {
        let by_series =
            spot_months_by_identity(&census::held_entries(std::slice::from_ref(vendor_census)));
        let mut keys: Vec<_> = by_series.keys().copied().collect();
        // SORTED SO THE ANSWER IS REPRODUCIBLE. A `HashMap`'s iteration order
        // varies per process and `from` ships the names that voted.
        keys.sort_unstable();
        for key in keys {
            let (exchange, segment, symbol) = key;
            // A stored calendar from another venue cannot decide which
            // minutes this requested exchange and segment owed. D-0660.
            if exchange.as_str() != asked.exchange || segment.as_str() != asked.segment {
                continue;
            }
            // THE AUDITED SERIES DOES NOT VOTE ON ITSELF. This one comparison
            // is the whole difference between a denominator and a tautology.
            if vendor_census.vendor == asked.vendor && symbol.as_str() == asked.symbol {
                continue;
            }
            let months: Vec<store::path::YearMonth> =
                by_series.get(&key).cloned().unwrap_or_default();
            let derived = crate::calendar_of::cached(
                &site.calendars,
                &site.store_root,
                vendor_census.vendor,
                exchange.as_str(),
                segment.as_str(),
                symbol.as_str(),
                stamps.modified(vendor_census),
                &months,
                census_holds(Some((vendor_census, key))),
            );
            // A PEER WHOSE HELD FILE DID NOT OPEN IS NAMED, NOT COUNTED AS A
            // PEER THAT TRADED NOTHING THERE. See `PeerCalendar::unreadable`.
            if !derived.unopened.is_empty() {
                unreadable.push(format!(
                    "{}:{}",
                    vendor_census.vendor.as_str(),
                    symbol.as_str()
                ));
                continue;
            }
            let calendar = derived.calendar;
            // A SERIES THIS FEED HOLDS NOTHING FOR IS NOT A VOTE FOR ANYTHING —
            // counting an empty calendar as agreement would let an absence
            // close the exchange.
            if calendar.sessions() > 0 {
                readings.push((
                    format!("{}:{}", vendor_census.vendor.as_str(), symbol.as_str()),
                    // CLONED HERE, NOT UNDER THE CACHE'S MUTEX. `cached` now
                    // hands back an `Arc` so a hit costs a refcount bump; the
                    // copy a reading genuinely needs happens on this thread
                    // with no lock held. See `calendar_of::Cache`.
                    (*calendar).clone(),
                ));
            }
        }
    }
    if readings.is_empty() {
        return PeerCalendar {
            calendar: None,
            from: Vec::new(),
            unreadable,
        };
    }
    let (agreed, _clashes) = crate::calendar_of::agree(&readings);
    let from: Vec<String> = readings.into_iter().map(|(name, _)| name).collect();
    PeerCalendar {
        calendar: Some(agreed),
        from,
        unreadable,
    }
}

/// How many months one audit request will walk before it stops and says so.
///
/// Twenty years at the store's own granularity. The operator's store spans 121
/// months, so this is not a limit anyone reaches by accident — it exists so a
/// hand-typed `to=9999-12` cannot walk a hundred thousand months while holding
/// a request open.
const MAX_AUDIT_MONTHS: usize = 240;

/// The month after this one, or `None` at the end of the representable range.
///
/// `YearMonth::new` is the only judge of what is representable, so December
/// rolls by asking rather than by a bound written here that could disagree
/// with it.
fn next_month(month: store::path::YearMonth) -> Option<store::path::YearMonth> {
    if month.month() >= 12 {
        store::path::YearMonth::new(month.year().checked_add(1)?, 1).ok()
    } else {
        store::path::YearMonth::new(month.year(), month.month().checked_add(1)?).ok()
    }
}

/// One month's verdict, before it is rendered.
struct AuditedMonth {
    /// Which month.
    month: store::path::YearMonth,
    /// Bars the verified schedule expects, including an entirely absent file.
    /// Zero is not a complete verdict when the input or schedule is unverified.
    expected: u32,
    /// Bars the store actually held.
    held: u32,
    /// Invalid minute-grid/order input; coverage is withheld, not complete.
    invalid_timestamps: u32,
    /// Minutes absent for a reason that is a real loss.
    lost: u32,
    /// Every absent minute, loss or not.
    absent: u32,
    /// Minutes this build makes no claim about — a day past the calendar's
    /// `LAST_DAY`, or a session whose length it does not know.
    ///
    /// **The number that explains a `held` larger than an `expected`.** Without
    /// it beside them, that pair reads as an arithmetic bug rather than as the
    /// calendar running out.
    unmeasured: u32,
    /// Why this month could not be opened at all, if it could not be.
    ///
    /// **A finding, not a failure.** Across a range, a month with no file is
    /// the loudest thing an audit can find: a backfill that never reached it
    /// leaves exactly this. Returning it as a `400` would end the walk at the
    /// first hole and report the absence as a malformed request.
    absent_file: Option<String>,
    /// Missing/corrupt dated session evidence is not a missing bar file.
    evidence_error: Option<String>,
    /// Records `bars::page` could not read.
    unreadable: usize,
    /// Whether `MAX_GAPS` stopped this month's walk.
    truncated: bool,
    /// The runs, already rendered.
    runs: String,
}

impl AuditedMonth {
    /// This month as one JSON object.
    fn render(&self) -> String {
        format!(
            r#"{{"month":{},"expected":{},"held":{},"lost_minutes":{},"absent_minutes":{},"unmeasured_minutes":{},"truncated":{},"unreadable_records":{},"invalid_timestamps":{},"absent_file":{},"evidence_error":{},"gaps":[{}]}}"#,
            render::json_string(&self.month.to_string()),
            self.expected,
            self.held,
            self.lost,
            self.absent,
            self.unmeasured,
            self.truncated,
            self.unreadable,
            self.invalid_timestamps,
            self.absent_file
                .as_ref()
                .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
            self.evidence_error
                .as_ref()
                .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
            self.runs,
        )
    }
}

/// Restore only local, receipted session metadata for this exact cash stock.
/// A gap-page read never downloads a master, widens a pull, or borrows another
/// instrument's flag. A missing date names its evidence error and leaves the
/// post-CAS denominator unmeasured; source bars remain readable and unchanged.
async fn audit_cash_schedule(
    site: &Site,
    asked: &Addressed,
    month: store::path::YearMonth,
) -> Result<Option<pull::cash_auction::Schedule>, String> {
    let first = Day::new(month.year(), month.month(), 1).map_err(|why| why.to_string())?;
    let window =
        pull::session::Window::new(first, first.end_of_month()).map_err(|why| why.to_string())?;
    audit_cash_schedule_window(site, asked, window).await
}

/// Audit one month of one series.
///
/// # The range comes from the MONTH, not from the bars
///
/// Deriving it from the first and last stored bar is the mistake that makes
/// this agree with itself and disagree with the store: a month whose first four
/// trading days never landed would report a clean interior and a perfect score,
/// because the missing days would fall outside a range they themselves defined.
fn audit_one(
    site: &Loaded,
    asked: &Addressed,
    month: store::path::YearMonth,
    calendar: Option<&pull::calendar::Calendar>,
    cash_schedule: Option<&pull::cash_auction::Schedule>,
) -> AuditedMonth {
    let empty = |absent_file: Option<String>| AuditedMonth {
        month,
        expected: 0,
        held: 0,
        invalid_timestamps: 0,
        lost: 0,
        absent: 0,
        unmeasured: 0,
        absent_file,
        evidence_error: None,
        unreadable: 0,
        truncated: false,
        runs: String::new(),
    };

    let Ok(first_day) = pull::session::Day::new(month.year(), month.month(), 1) else {
        return empty(Some(format!(
            "{month} is a month this build's calendar cannot name a first day \
             for, so there is no range to audit against"
        )));
    };
    let first = i64::from(first_day.days_from_epoch());
    let last = i64::from(first_day.end_of_month().days_from_epoch());

    // THE WHOLE MONTH, by index. `page` reads at a fixed stride, so this is
    // `n_valid` seeks of known length and nothing scans.
    let (rows, faults, absent_file) = match asked.open(site, month) {
        Ok(file) => {
            let n = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
            let (rows, faults) = bars::page(&file, 0, n);
            (rows, faults, None)
        }
        Err(why) => (Vec::new(), Vec::new(), Some(why)),
    };
    let stamps: Vec<i64> = rows.iter().map(|bar| bar.ts_micros).collect();
    // A MARKET SESSION AND AN INDEX-PUBLICATION WINDOW ARE NOT THE SAME FACT.
    // On 2021-02-24 SEBI proves 220 normal-market minutes, but also records
    // NIFTY computation unavailable during part of the still-trading morning
    // and gives no common interval for every stored index. The index-specific
    // door therefore refuses to turn that day into invented vendor holes;
    // CASH and derivative series keep the verified exchange-session calendar.
    let ledger = if asked.exchange == "NSE" && asked.segment == "CASH" && asked.contract.is_none() {
        pull::gaps::classify_cash(&stamps, first, last, cash_schedule)
    } else if asked.segment == "INDEX" {
        pull::gaps::classify_spot_index_against(&stamps, first, last, calendar)
    } else if asked.exchange == "NSE" && (asked.contract.is_some() || asked.segment == "FNO") {
        // A DERIVATIVES SERIES OWES THE DERIVATIVES VENUE'S DATED HOURS, the
        // ones `derive` folds it to (hunt-pull-1, D-1529): from 2026-08-03 a
        // 385-minute day, so the ten minutes after 15:30 are audited too.
        pull::gaps::classify_derivative_against(&stamps, first, last, calendar)
    } else {
        pull::gaps::classify_against(&stamps, first, last, calendar)
    };

    // RUNS, NOT MINUTES, and the reason travels with each one. 1,176 of 1,204
    // measured absences were four contiguous events; a per-minute list is the
    // same facts 43× longer.
    let runs = ledger
        .gaps
        .iter()
        .map(|gap| {
            format!(
                r#"{{"day":{},"from":{},"to":{},"minutes":{},"reason":{}}}"#,
                gap.day,
                gap.from,
                gap.to,
                gap.minutes(),
                render::json_string(gap.reason.word())
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    AuditedMonth {
        month,
        expected: ledger.expected,
        held: ledger.held,
        invalid_timestamps: ledger.invalid_timestamps,
        lost: ledger.lost_minutes(),
        absent: ledger.absent_minutes(),
        unmeasured: ledger.unmeasured_minutes(),
        // A FAULTY RECORD IS NOT SILENTLY SKIPPED, and here it matters more
        // than on the drawing route: a bar `page` could not read is a bar this
        // audit did not see, so it is scored as a HOLE. Reported beside the
        // verdict so "the file is damaged" cannot be read as "the vendor is
        // missing minutes" — opposite faults wanting opposite fixes.
        absent_file,
        evidence_error: None,
        unreadable: faults.len(),
        truncated: ledger.truncated,
        runs,
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::bars_json;
    use crate::server::tests::{
        cash_month_files, cash_month_replay_fixture, day, masters, store_root,
    };
    use brutex_core::vendor::Vendor;

    #[tokio::test]
    async fn cash_gap_page_uses_receipted_local_flags_and_refuses_missing_or_corrupt_evidence() {
        let (site, _, _) = cash_month_replay_fixture("cash-gap-clock").await;
        let site = std::sync::Arc::new(site);
        let before = cash_month_files(&site);
        let asked = Addressed::parse(
            "feed=zerodha&exchange=NSE&segment=CASH&symbol=RELIANCE&timeframe=1min&month=2026-08",
        )
        .unwrap();
        let cache = site.store_root.join("session-masters");
        let sample = std::fs::read(cache.join("NSE_CM_security_03082026.csv.gz")).unwrap();
        let missing = super::audit_cash_schedule(&site, &asked, asked.month)
            .await
            .unwrap_err();
        assert!(missing.contains("missing locally"), "{missing}");
        assert!(
            !cache.join("NSE_CM_security_04082026.csv.gz").exists(),
            "a GET cannot fetch missing metadata"
        );
        let uri: axum::http::Uri = "/gaps.json?feed=zerodha&exchange=NSE&segment=CASH&symbol=RELIANCE&timeframe=1min&month=2026-08".parse().unwrap();
        let (code, _, unknown) = gaps_json(axum::extract::State(site.clone()), uri.clone()).await;
        assert_eq!(code, axum::http::StatusCode::OK);
        assert!(unknown.contains("missing locally"), "{unknown}");
        assert!(
            unknown.contains(r#""held":720"#),
            "source still visible: {unknown}"
        );
        let first = day(2026, 8, 1);
        let mut sessions = 0;
        for number in first.days_from_epoch()..=first.end_of_month().days_from_epoch() {
            if pull::calendar::kind_of(i64::from(number))
                == pull::calendar::DayKind::Open(pull::calendar::Session::full())
            {
                let date = Day::from_days(number).unwrap();
                pull::cash_session_cache::install(&cache, date, &sample).unwrap();
                sessions += 1;
            }
        }
        let (_, _, checked) = gaps_json(axum::extract::State(site.clone()), uri.clone()).await;
        assert!(
            checked.contains(&format!(r#""expected":{}"#, sessions * 360)),
            "{checked}"
        );
        assert!(
            checked.contains(&format!(r#""lost_minutes":{}"#, sessions * 360 - 720)),
            "whole absent source days stay visible: {checked}"
        );
        assert!(checked.contains(r#""evidence_error":null"#), "{checked}");
        assert!(
            checked.contains(r#""source":"table""#),
            "cash peers do not set the close: {checked}"
        );
        let wrong = Addressed { symbol: "TCS".to_owned(), ..Addressed::parse("feed=zerodha&exchange=NSE&segment=CASH&symbol=RELIANCE&timeframe=1min&month=2026-08").unwrap() };
        let identity = super::audit_cash_schedule(&site, &wrong, wrong.month)
            .await
            .unwrap_err();
        assert!(identity.contains("no EQ mapping"), "{identity}");
        std::fs::write(
            cache.join("NSE_CM_security_04082026.csv.gz.receipt"),
            "damaged fixture receipt",
        )
        .unwrap();
        let (_, _, corrupt) = gaps_json(axum::extract::State(site.clone()), uri).await;
        assert!(
            !corrupt.contains(r#""evidence_error":null"#),
            "a previously read master does not hide disk damage: {corrupt}"
        );
        assert!(corrupt.contains("receipt"), "{corrupt}");
        assert_eq!(
            cash_month_files(&site),
            before,
            "auditing never edits source or derived bars"
        );
    }

    #[tokio::test]
    async fn minute_gap_route_refuses_daily_or_coarse_rows_as_minute_evidence() {
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-rung", None, None),
            &store_root("gaps-rung"),
        ));
        for rung in ["1day", "5min", "60min"] {
            let uri = format!("/gaps.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe={rung}&month=2026-08").parse().unwrap();
            let (code, _, body) = gaps_json(axum::extract::State(site.clone()), uri).await;
            assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{body}");
            assert!(body.contains("requires timeframe=1min"), "{body}");
        }
    }

    /// probeapi-2, D-1200: an address the store refuses is a 400 on the gaps
    /// route exactly as on the bars route — never a 200 "vendor-hole" report
    /// about a path that cannot exist. A valid address whose month is simply
    /// not held keeps answering 200 with the absence named.
    #[tokio::test]
    async fn gaps_route_refuses_an_invalid_store_address_like_the_bars_route() {
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-invalid-address", None, None),
            &store_root("gaps-invalid-address"),
        ));
        let rest = "timeframe=1min&month=2026-01";
        let refused = [
            // The probe's own query: every segment empty.
            "month=2026-01".to_owned(),
            format!("feed=zerodha&{rest}"),
            format!("feed=zerodha&exchange=NSE&segment=INDEX&{rest}"),
            format!("feed=zerodha&exchange=..&segment=INDEX&symbol=NIFTY&{rest}"),
            format!("feed=zerodha&exchange=NSE&segment=..&symbol=NIFTY&{rest}"),
            format!("feed=zerodha&exchange=NSE&segment=INDEX&symbol=..&{rest}"),
            format!("feed=zerodha&exchange=NSE&segment=INDEX&symbol=.&{rest}"),
            format!("feed=zerodha&exchange=NSE&segment=INDEX&symbol=A%2FB&{rest}"),
            format!("feed=zerodha&exchange=nse&segment=INDEX&symbol=NIFTY&{rest}"),
            format!(
                "feed=zerodha&exchange=NSE&segment=INDEX&symbol={}&{rest}",
                "N".repeat(4096)
            ),
            // A RANGE is checked once, before any month is walked.
            format!("feed=zerodha&exchange=NSE&segment=INDEX&symbol=..&{rest}&to=2026-12"),
        ];
        for query in refused {
            let uri: axum::http::Uri = format!("/gaps.json?{query}").parse().unwrap();
            let (code, _, gaps) = gaps_json(axum::extract::State(site.clone()), uri.clone()).await;
            assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{query}: {gaps}");
            assert!(gaps.contains("path segment"), "{query}: {gaps}");
            assert!(!gaps.contains("vendor-hole"), "{query}: {gaps}");
            assert!(!gaps.contains("lost_minutes"), "{query}: {gaps}");
            // ONE VOICE: the bars route refuses the same query with the same
            // status, so the two pages cannot disagree about one address.
            let (bars_code, _, _) = bars_json(axum::extract::State(site.clone()), uri).await;
            assert_eq!(bars_code, axum::http::StatusCode::BAD_REQUEST, "{query}");
        }
        // The control: a valid address that is not held is still a finding.
        let uri: axum::http::Uri =
            format!("/gaps.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NOSUCH&{rest}")
                .parse()
                .unwrap();
        let (code, _, held) = gaps_json(axum::extract::State(site.clone()), uri).await;
        assert_eq!(code, axum::http::StatusCode::OK, "{held}");
        assert!(
            held.contains("does not exist") || held.contains("absent"),
            "{held}"
        );
    }

    #[test]
    fn absent_minute_file_retains_calendar_obligation_like_an_empty_file() {
        let root = store_root("gap-absent-obligation");
        let site = std::sync::Arc::new(Site::serving(
            &masters("gap-absent-obligation", None, None),
            &root,
        ));
        let asked = Addressed::parse(
            "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2024-01",
        )
        .unwrap();
        let date = i64::from(day(2024, 1, 1).days_from_epoch());
        let calendar =
            pull::calendar::Calendar::from_observed(&[pull::calendar::Observed::from_runs(
                date,
                &[(555, 929)],
            )]);
        let absent = audit_one(&site, &asked, asked.month, Some(&calendar), None);
        assert!(absent.absent_file.is_some());
        assert_eq!(absent.held, 0);
        assert_eq!(absent.expected, 375);
        assert_eq!(absent.lost, 375);
        assert!(
            absent
                .runs
                .contains(r#""from":555,"to":929,"minutes":375,"reason":"vendor-hole""#)
        );
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month: asked.month,
            file: store::path::FileKind::Bars,
        })
        .unwrap();
        let id =
            u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
        drop(store::file::BarFile::open_or_create(&root, path, id).unwrap());
        let empty = audit_one(&site, &asked, asked.month, Some(&calendar), None);
        assert!(empty.absent_file.is_none());
        assert_eq!(empty.expected, absent.expected);
        assert_eq!(empty.lost, absent.lost);
        assert_eq!(empty.runs, absent.runs);
    }

    /// **ONLY AN NSE DERIVATIVES SERIES IS AUDITED AGAINST THE DERIVATIVES
    /// VENUE'S HOURS.** From 2026-08-03 that venue trades to 15:40, a 385-minute
    /// day where the exchange session is 375. An NSE futures series, by
    /// segment or by contract, owes the ten extra minutes; the same series
    /// named on another exchange, and an NSE series that is neither, owe the
    /// exchange session alone. Each address below is absent from the store, so
    /// the whole obligation is the classifier's and nothing else is measured.
    #[test]
    fn only_an_nse_derivatives_series_owes_the_derivatives_venue_hours() {
        let root = store_root("gap-derivative-door");
        let site = std::sync::Arc::new(Site::serving(
            &masters("gap-derivative-door", None, None),
            &root,
        ));
        let date = i64::from(day(2026, 8, 3).days_from_epoch());
        let calendar =
            pull::calendar::Calendar::from_observed(&[pull::calendar::Observed::from_runs(
                date,
                &[(555, 929)],
            )]);
        for (query, owed) in [
            ("exchange=NSE&segment=FNO&symbol=NIFTY", 385),
            ("exchange=BSE&segment=FNO&symbol=SENSEX", 375),
            ("exchange=NSE&segment=SPOT&symbol=NIFTY", 375),
        ] {
            let asked = Addressed::parse(&format!(
                "feed=zerodha&{query}&timeframe=1min&month=2026-08"
            ))
            .unwrap();
            let audit = audit_one(&site, &asked, asked.month, Some(&calendar), None);
            assert!(audit.absent_file.is_some(), "{query}");
            assert_eq!(audit.expected, owed, "{query}");
            assert_eq!(audit.lost, owed, "{query}");
        }
    }

    #[test]
    fn committed_off_grid_minutes_do_not_certify_the_gap_page() {
        for (tag, extra) in [("gap-shifted", false), ("gap-extra-stamp", true)] {
            let root = store_root(tag);
            let site = std::sync::Arc::new(Site::serving(&masters(tag, None, None), &root));
            let asked = Addressed::parse(
                "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2024-01",
            )
            .unwrap();
            let date = i64::from(day(2024, 1, 1).days_from_epoch());
            let at = |minute: i64| (date * 86_400 - 19_800 + minute * 60) * 1_000_000;
            let mut rows: Vec<_> = (555..930)
                .map(|minute| store::format::Bar {
                    ts_micros: at(minute) + if extra { 0 } else { 30_000_000 },
                    open: 100,
                    high: 110,
                    low: 90,
                    close: 105,
                    volume: 1,
                    open_interest: i64::MIN,
                })
                .collect();
            if extra {
                rows.insert(
                    1,
                    store::format::Bar {
                        ts_micros: at(555) + 30_000_000,
                        ..rows[0]
                    },
                );
            }
            let path = store::path::StorePath::new(store::path::PathParts {
                vendor: Vendor::Zerodha,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: store::path::Timeframe::MINUTE_1,
                month: asked.month,
                file: store::path::FileKind::Bars,
            })
            .unwrap();
            let id =
                u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
            // FORGED AS A PRE-D-0915 FILE. The write boundary now refuses an
            // off-grid stamp (`StoreError::OffGrid`), so a 1min file holding
            // them can only be one written before that refusal existed — and
            // such files are what this reader's own defence still faces. Laid
            // by hand: unsealed (flags 0, the legacy shape the reader accepts
            // without a `.crc`), genesis in slot 0, the commit in slot 1.
            let genesis = store::header::Header::genesis_at(store::layout::Layout::V2, id, 60, 0); // unsealed is version 2 (D-1571)
            let committed = genesis
                .advance(
                    u64::try_from(rows.len()).unwrap(),
                    rows.first().unwrap().ts_micros,
                    rows.last().unwrap().ts_micros,
                )
                .unwrap();
            let mut image = vec![0u8; usize::try_from(store::format::HEADER_LEN).unwrap()];
            for commit in [genesis.commit().unwrap(), committed.commit().unwrap()] {
                let at = usize::try_from(commit.offset).unwrap();
                image
                    .iter_mut()
                    .skip(at)
                    .zip(commit.bytes)
                    .for_each(|(dst, src)| *dst = src);
            }
            for row in &rows {
                image.extend_from_slice(&row.image());
            }
            let source_path = path.to_path_buf(&root);
            std::fs::create_dir_all(source_path.parent().unwrap()).unwrap();
            std::fs::write(&source_path, &image).unwrap();
            assert_eq!(
                store::file::BarFile::open_existing(&root, path, id)
                    .unwrap()
                    .records(),
                u64::try_from(rows.len()).unwrap(),
                "the forged legacy month opens and holds every off-grid row"
            );
            let before = std::fs::read(&source_path).unwrap();
            let audit = audit_one(&site, &asked, asked.month, None, None);
            assert_eq!(audit.invalid_timestamps, if extra { 1 } else { 375 });
            assert_eq!(audit.expected, 0, "no coverage claim on invalid input");
            assert_eq!(audit.lost, 0, "no invented downstream holes");
            assert!(audit.render().contains(&format!(
                r#""invalid_timestamps":{}"#,
                audit.invalid_timestamps
            )));
            assert_eq!(
                std::fs::read(&source_path).unwrap(),
                before,
                "audit is read-only"
            );
        }
    }

    /// **THE INDEX AUDIT DOES NOT TURN A KNOWN SYSTEMS OUTAGE INTO VENDOR LOSS.**
    ///
    /// The shared calendar knows SEBI's 220 normal-market minutes on
    /// 2021-02-24. The same primary record says NIFTY computation was
    /// unavailable during part of the still-trading morning and does not give
    /// one exact publication window for every stored index. `audit_one` must
    /// therefore select the spot-index refusal door, not the equity-session
    /// classifier that would report 166 invented vendor holes beside the
    /// store's 54-bar prefix.
    #[test]
    fn an_index_audit_keeps_the_outage_day_unmeasured() {
        let root = store_root("gaps-index-outage");
        let month = store::path::YearMonth::new(2021, 2).expect("a legal month");
        let day = pull::calendar::SYSTEMS_OUTAGE_DAY;
        let at = |minute: u16| (day * 86_400 - 19_800 + i64::from(minute) * 60) * 1_000_000;
        let rows: Vec<store::format::Bar> = (555..=608)
            .map(|minute| store::format::Bar {
                ts_micros: at(minute),
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect();
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the store cross-check accepts the stable low 32 bits"
        )]
        let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let mut file =
            store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a file");
        file.append(&rows).expect("the observed prefix");
        drop(file);

        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-index-outage", None, None),
            &root,
        ));
        let asked = Addressed::parse(
            "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2021-02",
        )
        .expect("an index address");
        let calendar =
            pull::calendar::Calendar::from_observed(&[pull::calendar::Observed::from_runs(
                day,
                &[(555, 608)],
            )]);
        assert_eq!(calendar.expected_bars(day), Some(220));

        let verdict = audit_one(&site, &asked, month, Some(&calendar), None);
        assert_eq!(verdict.held, 54, "stored evidence remains visible");
        assert_eq!(verdict.expected, 0, "no invented index denominator");
        assert_eq!(verdict.lost, 0, "no invented vendor loss");
        assert!(
            verdict.unmeasured >= 1_440,
            "the outage day is explicitly unmeasured"
        );
        assert!(verdict.runs.contains(r#""reason":"unmeasured""#));
    }

    /// **AN ABSENT MONTH IS A FINDING, AND THE WALK DOES NOT STOP AT IT.**
    ///
    /// This is the assertion the ranged form exists for. A backfill that never
    /// reached March 2021 leaves no file at all, so across a range the absence
    /// IS the thing an operator is looking for — and the obvious implementation
    /// makes it a `400`, which ends the walk at the first hole and reports the
    /// finding as a malformed request. Every month after it then goes unlooked
    /// at and unmentioned, which reads as no news.
    ///
    /// Also pins the two refusals that must NOT be silent, because both fail in
    /// the direction that looks like health: a range running backwards, and an
    /// unparseable `to` collapsing to a single month. The second is the subtler
    /// one — a narrowed range answers a different question and reports the
    /// months it never opened as simply absent from the answer.
    #[tokio::test]
    async fn a_month_with_no_file_is_reported_and_the_range_walks_past_it() {
        let root = store_root("gapsrange");
        // ONE MONTH ON DISK, THREE ASKED FOR. February and March were never
        // pulled, which is exactly the state a stopped backfill leaves.
        let month = store::path::YearMonth::new(2024, 1).expect("a legal month");
        let day = i64::from(
            pull::session::Day::new(2024, 1, 1)
                .expect("a legal day")
                .days_from_epoch(),
        );
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the cross-check `open` folds; any 32 bits serve"
        )]
        let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let mut file =
            store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a bar file");
        file.append(&[store::format::Bar {
            ts_micros: (day * 86_400 - 19_800 + 570 * 60) * 1_000_000,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }])
        .expect("one bar");
        drop(file);

        let site = std::sync::Arc::new(Site::serving(&masters("gapsrange", None, None), &root));
        let ask = |q: &str| {
            let site = std::sync::Arc::clone(&site);
            let uri: axum::http::Uri = format!(
                "/gaps.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY\
                 &timeframe=1min&month=2024-01{q}"
            )
            .parse()
            .expect("a uri");
            async move { gaps_json(axum::extract::State(site), uri).await }
        };

        let (code, _, three) = ask("&to=2024-03").await;
        assert_eq!(code, axum::http::StatusCode::OK, "{three}");
        assert!(
            three.contains(r#""months":3"#) && three.contains(r#""months_absent":2"#),
            "all three months are looked at and the two with no file are \
             COUNTED — a walk that stopped at the first would report one: {three}"
        );
        assert!(
            three.contains(r#""month":"2024-03""#),
            "the walk reached the far end rather than ending at the hole: {three}"
        );
        assert!(
            three.contains(r#""month":"2024-01","expected":"#),
            "and the month that IS on disk still carries a real verdict: {three}"
        );

        // WITHOUT `to`, THE RANGE IS ONE MONTH. The single-month question must
        // stay a single-month question, or every existing caller silently
        // starts auditing a range it never asked for.
        let (_, _, one) = ask("").await;
        assert!(
            one.contains(r#""months":1"#) && one.contains(r#""months_absent":0"#),
            "an absent `to` is not a range: {one}"
        );

        // BACKWARDS REFUSES rather than answering an empty walk, which would be
        // indistinguishable from a clean store.
        let (back_code, _, back) = ask("&to=2023-12").await;
        assert_eq!(back_code, axum::http::StatusCode::BAD_REQUEST, "{back}");
        assert!(back.contains("backwards"), "and it says why: {back}");

        // AND AN UNREADABLE `to` REFUSES rather than collapsing to one month.
        // This is the failure that looks like success: eleven months would go
        // unopened and unmentioned, and the answer would read as complete.
        //
        // `notamonth` rather than `2024-1`, and the difference is worth
        // recording because it caught this test out: **`2024-1` IS January**.
        // `month_param` splits on the hyphen and parses each side as a number,
        // so a missing leading zero is a legal spelling — the same one
        // `/bars.json` and the ranged bars route have always accepted. Making
        // this route stricter than its neighbours would refuse links they
        // still write.
        let (typo_code, _, typo) = ask("&to=notamonth").await;
        assert_eq!(typo_code, axum::http::StatusCode::BAD_REQUEST, "{typo}");
        assert!(
            typo.contains("YYYY-MM") && typo.contains("to"),
            "naming both the shape it wanted and WHICH parameter: {typo}"
        );

        // THE CALENDAR'S COVERAGE IS IN EVERY ANSWER, because it bounds every
        // number in one. `expected` counts only days the table can vouch for,
        // so a month running past `LAST_DAY` reports `held` LARGER than
        // `expected` — measured on the operator's store the day this shipped:
        // NIFTY at one minute for 2026-08 answered `expected 5,625, held
        // 7,500, lost 0`, twenty trading days of bars against fifteen vouched
        // for, because the table ends 2026-08-21.
        //
        // Read without the bound beside it that pair is an arithmetic bug, and
        // an operator right to distrust it has no way to learn that it is not
        // one. Asserted rather than left to a reader, because NOTHING ELSE in
        // this workspace notices the table expiring: `LAST_DAY` appears outside
        // `pull::calendar` exactly once, in that module's own test.
        // NO DAILY RUNG IN THIS FIXTURE, SO NO CALENDAR — AND THAT IS THE
        // ANSWER, NOT A ZERO.
        //
        // `calendar_of` derives which days traded from the `1day` rung,
        // deliberately: a minute rung validating itself would make a failed pull
        // read as a holiday. A series stored without its daily rung therefore
        // yields NO calendar at all, every minute falls to `Unmeasured`, and
        // `expected` is 0 beside a `held` of however many bars are there.
        //
        // Reported as `days: 0, covers_span: false` rather than as a clean
        // sheet, which is the §4 distinction: this is a question the build
        // cannot answer, not a store that owes nothing. An operator shown
        // `lost_minutes: 0` with no other signal would read it as health.
        // NOTHING INDEPENDENT TO VOTE, SO THE TABLE ANSWERS — AND IT SAYS SO.
        //
        // This store holds one feed's copy of one symbol, so `peer_calendar`
        // finds no reading that is not the series under audit. Deriving a
        // calendar from that series would answer "no losses" for every possible
        // input; the typed table is independent of the bars, so it reports the
        // month as almost entirely missing, which it is.
        assert!(
            three.contains(r#""source":"table""#) && three.contains(r#""voted_by":[]"#),
            "with no peer to vote the table answers, and the response names it \
             rather than leaving a number un-attributable: {three}"
        );
        assert!(
            three.contains(r#""lost_minutes":8249"#),
            "one bar stored against a month the calendar owes 8,250 — the table \
             is independent of the bars, so it can still say that: {three}"
        );
        let (short_code, _, short) = ask("&to=2024-1").await;
        assert_eq!(
            short_code,
            axum::http::StatusCode::OK,
            "an unpadded month is the same month: {short}"
        );
        assert!(
            short.contains(r#""months":1"#),
            "2024-1 is January, so the range is one month long: {short}"
        );
    }
}
