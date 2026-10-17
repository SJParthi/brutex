//! `/gaps.json` -- whether one stored series is complete across a range of
//! months, judged against the calendar of record (D-3336), or against a
//! calendar its peers vote for when `calendar=peers` asks for one.
//!
//! # Where it lives
//!
//! Cut out of `server.rs` by the WP-01 seam and mounted there as a child
//! module, so it reads that file's private helpers through `super::` and
//! `server.rs` imports each name back.
//!
//! # What D-3336 changed (WP-35)
//!
//! - The default denominator is the calendar of record, which no stored bar
//!   moves: the peer union read a hole every index shares as "full" (Zerodha
//!   NIFTY 1min 2020-12: `lost 0` beside 15,840 unmeasured minutes before,
//!   4,125 lost minutes named after, on a shadow of the operator's store).
//! - `timeframe=1s` is audited (DB-073): folded to minutes with `pull::fold`,
//!   an index's seconds audited against the 22,500 a session owes, and a
//!   trade-driven share or option measured by the sessions it printed on.
//! - A halt's minutes are counted apart, a contract's minutes after its expiry
//!   are neither owed nor lost, and a series named without its exchange and
//!   segment is found in the census or refused by name.

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
/// # Cost, stated rather than claimed (DB-A30, R3-06)
///
/// **Per month asked: one month summary read, and no bar record**, when the
/// month keeps one that describes its committed header
/// (`store::month_summary`): one open, one read of at most a few tens of
/// kilobytes, one CRC, then one pass over the month's minutes (at most 44,640)
/// and, for a spot index at one second, over each day's owed seconds (at most
/// 86,400 a day). None of it moves with the bars a month holds, so a NIFTY
/// one-second month of 400,000 bars costs what one of 1,024 does. A month
/// with no usable summary (none kept, torn, stale, under 1,024 bars, sparse
/// seconds an index audit needs, input the fold may refuse) has its bars read
/// whole, as before, and the answer names why (`summary_fallback`) and how
/// many records it read (`bar_records_read`); a page that sees `built` sees
/// the slow path, never a silent one.
///
/// **Per request, linear in the months asked**, capped at 12 one-second
/// months and 240 minute months (`asked_range`) and audited off the async
/// workers on the calendar pool. `MAX_GAPS` bounds the answer's SIZE, and
/// `Ledger::truncated` says so out loud when it bites -- a truncated ledger
/// that read like a complete one would report a store healthier than it is.
/// That is not O(1) per request and is not claimed to be: it is O(months
/// asked), each month O(1) in its bars, and gate 8 (`benches/ratio/gaps.rs`)
/// holds both shapes.
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

    let asked = match Addressed::parse(query) {
        Ok(asked) => asked,
        Err(why) => return refuse(why),
    };
    // A SERIES NAMED WITHOUT ITS EXCHANGE AND SEGMENT IS FOUND IN THE CENSUS,
    // or refused by name: see `inferred`.
    let (asked, inferred) = match inferred(&site, asked).await {
        Ok(Ok(found)) => found,
        Ok(Err(why)) => return refuse(why),
        Err(why) => return calendar_admission_refused(&why),
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
    // ONE MINUTE OR ONE SECOND, AND THE REFUSAL SAYS WHICH WAY IS WHICH.
    //
    // This refused `1s` with "coarser bars cannot prove their source minutes":
    // one second is FINER than a minute, and GDFL holds nothing else (D-2802),
    // so none of its series could be audited at all (DB-073). A `1s` month is
    // now folded to minutes with `pull::fold`, the one fold authority, and
    // audited second by second where seconds are owed (D-3336 §4).
    let one_second = asked.timeframe == store::path::Timeframe::SECOND_1;
    if asked.timeframe != store::path::Timeframe::MINUTE_1 && !one_second {
        return refuse(format!(
            "minute coverage requires timeframe=1min or timeframe=1s; {} bars are coarser \
             than a minute and cannot prove their source minutes (1s is finer, and is folded \
             to minutes with pull::fold)",
            asked.timeframe.as_str()
        ));
    }
    let (mode, min_run, span, truncated_range) = match asked_range(query, &asked, one_second) {
        Ok(parsed) => parsed,
        Err(why) => return refuse(why),
    };

    // THE DENOMINATOR, ONCE FOR THE SPAN — never the series being audited. A
    // calendar derived from the audited bars reads a hole as a window edge
    // (measured: one minute deleted mid-session answered `lost 0`). The
    // calendar of record is independent of every bar in the store, so it is
    // the default (D-3336); the union of peers stays one parameter away, and
    // an NSE cash stock consults neither, because a peer's auction
    // eligibility is not this stock's.
    // A ONE-SECOND SHARE IS TRADE-DRIVEN AND OWES NO COUNT (D-3336 §2), so its
    // auction eligibility decides nothing and the record is its calendar.
    let cash = asked.exchange == "NSE"
        && asked.segment == "CASH"
        && asked.contract.is_none()
        && !one_second;
    let denominator = if cash {
        Denominator::table()
    } else if mode == Mode::Record {
        Denominator::record(&span)
    } else {
        // THE PEER VOTE, KEPT OR BUILT OFF THE REQUEST PATH (DB-N19): see
        // `voted`. A build is derived on the calendar pool behind its
        // admission (W1-api2-11, D-1443) and waited for at most the served
        // budget, past which the request is answered "building".
        match voted(&site, &asked).await {
            Ok(peers) => Denominator::peers(peers),
            Err(answer) => return answer,
        }
    };
    let (months, denominator) =
        match audit_span(&site, asked, span.clone(), denominator, min_run).await {
            Ok(done) => done,
            Err(why) => return calendar_admission_refused(&why),
        };

    (
        axum::http::StatusCode::OK,
        json(),
        gaps_body(&months, &span, &denominator, truncated_range, &inferred),
    )
}

/// `/gaps.json?{query}` answered by the route's own handler over `site`: its
/// status code and body, nothing else. The door gate 8 times
/// (`crates/api/benches/ratio/gaps.rs`, R3-06), which cannot name the private
/// handler; no second code path.
pub async fn gaps_answer(site: Loaded, query: &str) -> (u16, String) {
    let uri = match format!("/gaps.json?{query}").parse::<axum::http::Uri>() {
        Ok(uri) => uri,
        Err(why) => {
            return (
                400,
                format!(r#"{{"error":{}}}"#, render::json_string(&why.to_string())),
            );
        }
    };
    let (code, _, body) = gaps_json(axum::extract::State(site), uri).await;
    (code.as_u16(), body)
}

/// The identity a `/gaps.json` answer audited, rendered: the address the
/// request named, or the one [`inferred`] found for it, and which.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity {
    /// The rendered `identity` member, without its leading comma.
    json: String,
    /// Whether the audit is at one second.
    one_second: bool,
}

/// `asked`, with its exchange and segment found in the feed's census when the
/// request named neither, and the identity the answer will name.
///
/// # Why a request may leave them out
///
/// `/gaps.json?feed=gdfl&symbol=NIFTY&timeframe=1s&month=2018-09` names one
/// series: GDFL holds NIFTY's one-second spot bars under exactly one exchange
/// and segment. Refusing it for an empty `segment` (probeapi-2, D-1200) was
/// right while the alternative was auditing a path that can never exist; the
/// census can say instead which path does. So a request naming neither
/// `exchange` nor `segment` is answered for the ONE identity the feed's census
/// holds for the symbol, the contract and the rung at the first month asked,
/// found by probing the six exchange-and-segment pairs: six hash probes, no
/// walk. The answer says it was inferred.
///
/// **Never a guess.** No identity held is refused, and so is more than one
/// (`NIFTY` as an index and as a share would be two series): the request is
/// told which to name. A request naming either word is taken as written, and
/// an empty one there is still refused by the store path.
///
/// The census is read on the calendar pool, off the async workers
/// (W1-api2-11, D-1443).
///
/// # Errors
///
/// The pool's refusal; inside it, the refusal sentence for an identity that
/// cannot be found.
async fn inferred(
    site: &Loaded,
    asked: Addressed,
) -> Result<Result<(Addressed, Identity), String>, crate::detail::RunError> {
    let one_second = asked.timeframe == store::path::Timeframe::SECOND_1;
    let rendered = move |asked: &Addressed, inferred: bool| Identity {
        json: format!(
            r#""identity":{{"exchange":{},"segment":{},"symbol":{},"contract":{},"inferred":{inferred}}}"#,
            render::json_string(&asked.exchange),
            render::json_string(&asked.segment),
            render::json_string(&asked.symbol),
            asked.contract.as_ref().map_or_else(
                || "null".to_owned(),
                |contract| render::json_string(contract.as_str())
            ),
        ),
        one_second,
    };
    if !asked.exchange.is_empty() || !asked.segment.is_empty() || asked.symbol.is_empty() {
        let identity = rendered(&asked, false);
        return Ok(Ok((asked, identity)));
    }
    let probe = std::sync::Arc::clone(site);
    crate::detail::run_calendar(move || {
        let mut asked = asked;
        let (exchange, segment) = identity_in_census(&probe, &asked, census_now_stamped)?;
        exchange.as_str().clone_into(&mut asked.exchange);
        segment.as_str().clone_into(&mut asked.segment);
        let identity = rendered(&asked, true);
        Ok((asked, identity))
    })
    .await
}

/// The one exchange and segment the feed's census holds `asked` under at its
/// first month and rung. See [`inferred`].
///
/// # Errors
///
/// The refusal sentence: a symbol the store cannot name, none held, or more
/// than one.
fn identity_in_census(
    site: &Site,
    asked: &Addressed,
    census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
) -> Result<
    (
        brutex_core::instrument::Exchange,
        brutex_core::instrument::Segment,
    ),
    String,
> {
    use brutex_core::instrument::{Exchange, Segment};
    let symbol = brutex_core::symbol::Symbol::new(&asked.symbol).map_err(|why| {
        format!(
            "{:?} is not a symbol the store can name ({why:?}); name exchange and segment \
             with it",
            asked.symbol
        )
    })?;
    let (_, (fresh, _)) = census(site);
    let own = fresh.iter().find(|row| row.vendor == asked.vendor);
    let mut held = Vec::new();
    for exchange in [Exchange::Nse, Exchange::Bse] {
        for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
            let key = census::Series {
                contract: asked.contract,
                exchange,
                segment,
                symbol,
                timeframe: asked.timeframe,
            }
            .at(asked.month);
            if own.is_some_and(|row| row.rows_for(&key).is_some()) {
                held.push((exchange, segment));
            }
        }
    }
    let what = format!(
        "{}{} {} {}",
        asked.symbol,
        asked
            .contract
            .as_ref()
            .map_or_else(String::new, |contract| format!("/{}", contract.as_str())),
        asked.timeframe.as_str(),
        asked.month,
    );
    match held.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!(
            "exchange and segment were not named, and {}'s census holds no {what} to find \
             them by; name both (e.g. exchange=NSE&segment=INDEX)",
            asked.vendor.as_str()
        )),
        many => Err(format!(
            "exchange and segment were not named, and {}'s census holds {what} as {} series \
             ({}); name which",
            asked.vendor.as_str(),
            many.len(),
            many.iter()
                .map(|(exchange, segment)| format!("{}/{}", exchange.as_str(), segment.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The denominator, the listing bound and the span a `/gaps.json` request
/// names, and whether the span's ceiling stopped it.
///
/// `to` IS OPTIONAL AND DEFAULTS TO `month`, so the single-month question
/// stays a single-month question and the range is opt-in. An unparseable `to`
/// REFUSES rather than collapsing to one month: a range that silently becomes
/// narrower answers a different question than the one asked, and reports the
/// months it never looked at as absent from the answer.
///
/// # Errors
///
/// The refusal sentence for the first parameter that is wrong.
fn asked_range(
    query: &str,
    asked: &Addressed,
    one_second: bool,
) -> Result<(Mode, u32, Vec<store::path::YearMonth>, bool), String> {
    let mode = match param(query, "calendar").as_str() {
        "" | "record" => Mode::Record,
        "peers" => Mode::Peers,
        other => {
            return Err(format!(
                "calendar={other:?} is not a denominator this route knows: calendar=record \
                 (the default: the calendar of record, D-3336) or calendar=peers (every other \
                 stored series on this exchange and segment, agreed by union)"
            ));
        }
    };
    let min_run = min_run_param(query)?;
    let last_month = if param(query, "to").is_empty() {
        asked.month
    } else {
        month_param(query, "to")?
    };
    if last_month < asked.month {
        return Err(format!(
            "the range runs backwards: {} is before {}",
            last_month, asked.month
        ));
    }
    let (span, truncated) = span_of(
        asked.month,
        last_month,
        if one_second {
            MAX_AUDIT_MONTHS_1S
        } else {
            MAX_AUDIT_MONTHS
        },
    );
    Ok((mode, min_run, span, truncated))
}

/// Which denominator a `/gaps.json` request asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The calendar of record. The default (D-3336).
    Record,
    /// The union of every other stored series' calendar on the asked exchange
    /// and segment.
    Peers,
}

/// The shortest second-level run of missing seconds a `1s` audit lists by
/// default, in seconds.
///
/// A display bound, CHOSEN and not sourced: every missing second is counted
/// whatever the bound, and the longest run of each day is named whatever its
/// length. On GDFL's NIFTY 50 an isolated missing second is common (up to
/// about 900 runs on one day of 2018-09) and a 34-second stretch is rare, so
/// ten seconds lists the stretches without listing every isolated second.
const MIN_RUN_DEFAULT: u32 = 10;

/// The `min_run` a request names, or [`MIN_RUN_DEFAULT`].
///
/// # Errors
///
/// A value that is not a whole number of seconds from 1 to 3,600.
fn min_run_param(query: &str) -> Result<u32, String> {
    let raw = param(query, "min_run");
    if raw.is_empty() {
        return Ok(MIN_RUN_DEFAULT);
    }
    raw.parse::<u32>()
        .ok()
        .filter(|seconds| (1..=3_600).contains(seconds))
        .ok_or_else(|| format!("min_run={raw:?} is not a whole number of seconds from 1 to 3600"))
}

/// The months `first..=last`, at most `ceiling` of them, and whether the
/// ceiling stopped the walk.
///
/// NEVER A SILENT `take(N)`. A ceiling that stopped quietly would report the
/// months it never reached as no news, which is the direction that reads as
/// health.
fn span_of(
    first: store::path::YearMonth,
    last: store::path::YearMonth,
    ceiling: usize,
) -> (Vec<store::path::YearMonth>, bool) {
    let mut span = Vec::new();
    let mut cursor = first;
    loop {
        if span.len() >= ceiling {
            return (span, true);
        }
        span.push(cursor);
        if cursor >= last {
            return (span, false);
        }
        let Some(next) = next_month(cursor) else {
            return (span, false);
        };
        cursor = next;
    }
}

/// The calendar a `/gaps.json` audit counts against, and where it came from.
pub(super) struct Denominator {
    /// The calendar, or `None` where the typed table answers inside
    /// `pull::gaps` (an NSE cash stock, or a peer vote nobody cast).
    calendar: Option<pull::calendar::Calendar>,
    /// `record`, `peers` or `table`.
    source: &'static str,
    /// The first and last day the source can vouch for.
    bounds: (i64, i64),
    /// The `feed:symbol` readings that voted, for `peers`.
    from: Vec<String>,
    /// Every feed or peer that could not be read, for `peers`.
    unreadable: Vec<String>,
}

impl Denominator {
    /// The typed table inside `pull::gaps`: [`pull::calendar::kind_of`].
    const fn table() -> Self {
        Self {
            calendar: None,
            source: "table",
            bounds: (pull::calendar::FIRST_DAY, pull::calendar::LAST_DAY),
            from: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    /// The calendar of record over `span`'s days. D-3336.
    fn record(span: &[store::path::YearMonth]) -> Self {
        let day = |month: &store::path::YearMonth| {
            pull::session::Day::new(month.year(), month.month(), 1).ok()
        };
        let first = span
            .first()
            .and_then(day)
            .map_or(0, |d| i64::from(d.days_from_epoch()));
        let last = span
            .last()
            .and_then(day)
            .map_or(0, |d| i64::from(d.end_of_month().days_from_epoch()));
        Self {
            calendar: Some(record_calendar(first, last)),
            source: "record",
            bounds: (
                pull::calendar::SOURCED_FIRST_DAY,
                pull::calendar::SOURCED_LAST_DAY,
            ),
            from: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    /// The peers' agreed calendar, or the table when nobody voted.
    fn peers(peers: PeerCalendar) -> Self {
        match peers.calendar {
            Some(calendar) => Self {
                bounds: (calendar.first_day(), calendar.last_day()),
                calendar: Some(calendar),
                source: "peers",
                from: peers.from,
                unreadable: peers.unreadable,
            },
            None => Self {
                from: peers.from,
                unreadable: peers.unreadable,
                ..Self::table()
            },
        }
    }
}

/// The calendar of record from `first` to `last` as a [`pull::calendar::Calendar`]
/// for `pull::gaps` to count against (D-3336).
///
/// Every session is the record's ([`pull::calendar::exchange_kind_of`]),
/// special windows included, with a recorded halt's whole minutes carved out
/// of it ([`carved`]): a halted minute is neither owed nor missing. A day the
/// record does not cover is unmeasured. The span reaches back and forward to
/// the nearest session within a fortnight, because a `Calendar` reads a day
/// outside its first and last session as unmeasured, and a month that opens on
/// a weekend would otherwise read its first closed days as unmeasured rather
/// than closed.
///
/// One `exchange_kind_of` per day of the span and one 1,440-minute pass per
/// halt day; no store read.
fn record_calendar(first: i64, last: i64) -> pull::calendar::Calendar {
    use pull::calendar::DayKind;
    let open = |day: i64| {
        matches!(
            pull::calendar::exchange_kind_of(day),
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured
        )
    };
    let low = (first.saturating_sub(14)..=first)
        .rev()
        .find(|day| open(*day))
        .unwrap_or(first);
    let high = (last..=last.saturating_add(14))
        .find(|day| open(*day))
        .unwrap_or(last);
    let mut observed = Vec::new();
    let mut unmeasured = Vec::new();
    for day in low..=high {
        match pull::calendar::exchange_kind_of(day) {
            DayKind::Open(session) => observed.push(pull::calendar::Observed {
                day,
                session: carved(day, session),
            }),
            DayKind::OpenLengthUnmeasured => {
                observed.push(pull::calendar::Observed { day, session: None });
            }
            DayKind::Closed => {}
            DayKind::Unmeasured => unmeasured.push(day),
        }
    }
    let mut calendar = pull::calendar::Calendar::from_observed(&observed);
    for day in unmeasured {
        calendar.withhold_closed(day, day);
    }
    calendar
}

/// `session` with `day`'s recorded halt carved out of its windows, or `None`
/// (a session of unknown length, which owes nothing and loses nothing) when
/// the carving would need more windows than a session holds.
fn carved(day: i64, session: pull::calendar::Session) -> Option<pull::calendar::Session> {
    let Some(halt) = pull::calendar::halt_on(day) else {
        return Some(session);
    };
    let mut runs: Vec<(u16, u16)> = Vec::new();
    for minute in 0..pull::calendar::MINUTES_PER_DAY {
        if !session.expects(minute) || halt.halted.holds(minute) {
            continue;
        }
        match runs.last_mut() {
            Some(last) if last.1.saturating_add(1) == minute => last.1 = minute,
            _ => runs.push((minute, minute)),
        }
    }
    if runs.len() > pull::calendar::MAX_WINDOWS {
        return None;
    }
    pull::calendar::Observed::from_runs(day, &runs).session
}

/// Every month of a `/gaps.json` span, audited OFF THE ASYNC WORKERS.
///
/// Each month's bar file and its dated cash-session evidence are read in the
/// calendar pool the peer vote already used, in one admission for the whole
/// span. The evidence reader is an `async fn` whose local variant never awaits
/// a fetch, but it reads and locks files inline, so it is driven to completion
/// on the blocking thread by the runtime's handle rather than on a worker.
/// `denominator` goes in and comes back, because the answer renders from it.
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
    denominator: Denominator,
    min_run: u32,
) -> Result<(Vec<AuditedMonth>, Denominator), crate::detail::RunError> {
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
                denominator.calendar.as_ref(),
                schedule.as_ref().ok().and_then(Option::as_ref),
                min_run,
            );
            audited.evidence_error = schedule.err();
            months.push(audited);
        }
        (months, denominator)
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
    denominator: &Denominator,
    truncated_range: bool,
    identity: &Identity,
) -> String {
    let one_second = identity.one_second;
    // THE ROLL-UP IS A SUM OF THE MONTHS, not a second computation over them. A
    // total that could disagree with the rows below it is a total nobody can
    // act on.
    let expected: u64 = months.iter().map(|m| u64::from(m.expected)).sum();
    let held: u64 = months.iter().map(|m| u64::from(m.held)).sum();
    let lost: u64 = months.iter().map(|m| u64::from(m.lost)).sum();
    let absent_files = months.iter().filter(|m| m.absent_file.is_some()).count();
    // HOW THE ANSWER WAS HAD, beside the answer (R3-06): months read off a
    // kept summary, months whose bars were read because none was usable (each
    // month row names why), and the bar records that cost. A store that keeps
    // no summaries answers the same numbers and says it read every bar.
    let summaries_kept = months
        .iter()
        .filter(|m| m.summary == SummarySource::Kept)
        .count();
    let summaries_built = months
        .iter()
        .filter(|m| m.summary == SummarySource::Built)
        .count();
    let records_read: u64 = months.iter().map(|m| m.records_read).sum();
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
    // claim is made either way — which makes `held` LARGER than `expected` for
    // any span running past the calendar's edge. `covers_span` says whether the
    // span ASKED for is inside what the source vouches for, and `stale` whether
    // today is past that source's last day.
    let (first_bound, last_bound) = denominator.bounds;
    let source = denominator.source;
    let day_text = |day: i64| {
        pull::session::Day::from_days(u32::try_from(day.max(0)).unwrap_or(u32::MAX)).map_or_else(
            |_| "null".to_owned(),
            |d| format!(r#""{:04}-{:02}-{:02}""#, d.year(), d.month(), d.day()),
        )
    };
    let stale = source != "peers"
        && ingest::today_ist().is_ok_and(|today| i64::from(today.days_from_epoch()) > last_bound);
    // AN ABSENT OR SHORT CALENDAR IS ITS OWN ANSWER, NOT A ZERO: see `covers_span`.
    let calendar_days = last_bound
        .saturating_sub(first_bound)
        .saturating_add(1)
        .max(0);
    let covers_span = span.first().is_some_and(|m| {
        pull::session::Day::new(m.year(), m.month(), 1)
            .is_ok_and(|d| i64::from(d.days_from_epoch()) >= first_bound)
    }) && span.last().is_some_and(|m| {
        pull::session::Day::new(m.year(), m.month(), 1)
            .is_ok_and(|d| i64::from(d.end_of_month().days_from_epoch()) <= last_bound)
    });
    let names = |list: &[String]| {
        list.iter()
            .map(|name| render::json_string(name))
            .collect::<Vec<_>>()
            .join(",")
    };
    let voted = names(&denominator.from);
    // A PEER THAT COULD NOT BE READ IS NAMED, not counted as a peer that held
    // nothing: see `PeerCalendar::unreadable`.
    let unreadable = names(&denominator.unreadable);
    let timeframe = if one_second { "1s" } else { "1min" };
    let fold = if one_second {
        r#","folded_with":"pull::fold, 1s to 1min, for the minute verdict; seconds audited where an index owes them (D-3336)""#
    } else {
        ""
    };

    format!(
        r#"{{{},"timeframe":"{timeframe}"{fold},"expected":{expected},"held":{held},"lost_minutes":{lost},"unmeasured_minutes":{unmeasured},"months":{},"months_absent":{absent_files},"summaries":{{"kept":{summaries_kept},"built":{summaries_built},"bar_records_read":{records_read}}},"truncated":{truncated_range},"calendar":{{"first":{},"last":{},"days":{calendar_days},"source":"{source}","stale":{stale},"covers_span":{covers_span},"voted_by":[{voted}],"unreadable":[{unreadable}]}},"month":[{rows}]}}"#,
        identity.json,
        months.len(),
        day_text(first_bound),
        day_text(last_bound),
    )
}

// THE PEER VOTE'S TYPE LIVES BESIDE THE CACHE THAT KEEPS IT, so a vote can be
// kept under the census stamps it was read under (WP-36, DB-N19). Its own doc
// says why the audited series may not vote.
pub(super) use crate::calendar_of::build::PeerCalendar;

/// Build a [`PeerCalendar`] for the series `asked` addresses, now, whatever
/// is kept: the reading a vote build makes ([`lead_vote`]), for a test.
#[cfg(test)]
pub(super) fn peer_calendar(site: &Loaded, asked: &Addressed) -> PeerCalendar {
    peer_calendar_reading(site, asked, census_now_stamped)
}

/// [`peer_calendar`], with the census it derives from passed in.
///
/// Production passes [`census_now_stamped`] and nothing else, for the reason
/// [`calendar_json_reading`](super::calendar_route::calendar_json_reading)
/// gives: a test installs a newer manifest after the census is read and before
/// a peer's calendar is kept. D-0695. Production reads it through
/// [`peer_vote_reading`], which also says whether the vote may be kept.
#[cfg(test)]
pub(super) fn peer_calendar_reading(
    site: &Site,
    asked: &Addressed,
    census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
) -> PeerCalendar {
    peer_vote_reading(site, asked, census).1
}

/// [`peer_calendar_reading`], and the stamps its vote may be kept under:
/// `None` when the census it read is not one its stamps could have read
/// (`read_as_stamped`, D-0695) or any feed or peer could not be read -- a vote
/// that names an unreadable peer is answered and built again next time, never
/// kept past the fault (DB-N19).
fn peer_vote_reading(
    site: &Site,
    asked: &Addressed,
    census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
) -> (Option<CensusStamps>, PeerCalendar) {
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
        // A stored calendar from another venue cannot decide which minutes
        // this requested exchange and segment owed (D-0660), and THE AUDITED
        // SERIES DOES NOT VOTE ON ITSELF: that one comparison is the whole
        // difference between a denominator and a tautology.
        keys.retain(|(exchange, segment, symbol)| {
            exchange.as_str() == asked.exchange
                && segment.as_str() == asked.segment
                && !(vendor_census.vendor == asked.vendor && symbol.as_str() == asked.symbol)
        });
        // HOW FAR A VOTE HAS GOT is what a "building" answer says (DB-N19).
        crate::calendar_of::build::owe(keys.len());
        for key in keys {
            let (exchange, segment, symbol) = key;
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
            crate::calendar_of::build::counted();
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
    let keep = (unreadable.is_empty() && super::census_cache::read_as_stamped(&stamps, &fresh))
        .then_some(stamps);
    if readings.is_empty() {
        return (
            keep,
            PeerCalendar {
                calendar: None,
                from: Vec::new(),
                unreadable,
            },
        );
    }
    let (agreed, _clashes) = crate::calendar_of::agree(&readings);
    let from: Vec<String> = readings.into_iter().map(|(name, _)| name).collect();
    (
        keep,
        PeerCalendar {
            calendar: Some(agreed),
            from,
            unreadable,
        },
    )
}

/// One `/gaps.json` answer: status, content type, body.
type GapsAnswer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// The peer vote's key: the parts of `asked` it reads.
fn vote_key(asked: &Addressed) -> crate::calendar_of::build::VoteKey {
    (
        asked.vendor,
        asked.exchange.clone(),
        asked.segment.clone(),
        asked.symbol.clone(),
    )
}

/// The peers' vote for the series `asked` addresses (DB-N19).
///
/// Kept under the census stamps of every vendor's manifest -- the vote reads
/// every feed's census -- and handed out while none has moved: one `stat` per
/// vendor and one probe. Otherwise one build per series, led or joined, run
/// on the calendar pool and waited for at most the served budget; past it the
/// request is answered `202` `"building"` and the build goes on. Before this
/// the first `calendar=peers` audit after a start derived every peer series'
/// calendar on the request (17.9 s for Zerodha NIFTY on WP-36's shadow) and
/// every later one walked, sorted and agreed them all again (0.26-0.40 s).
///
/// # Errors
///
/// The answer the request is given instead: `"building"`, a full build map,
/// or the admission refusal the build met.
async fn voted(site: &Loaded, asked: &Addressed) -> Result<PeerCalendar, GapsAnswer> {
    use crate::calendar_of::build::Joined;
    let builds = &site.calendars.builds;
    let key = vote_key(asked);
    let now = super::census_cache::manifest_stamps(&site.store_root);
    if let Some(vote) = builds.votes.kept(&key, &now) {
        return Ok((*vote).clone());
    }
    let build = match builds
        .votes
        .join(&key, Some(&now), builds.census_at_start())
    {
        Joined::Parked(voted) => return vote_of(voted),
        Joined::CensusAtStart(since) => {
            return Err(vote_building(
                asked, "census", "start-up", since, None, 0, 0,
            ));
        }
        Joined::Full => return Err(votes_full()),
        Joined::Follow(build) => build,
        Joined::Lead(build) => {
            lead_vote(site, asked, &build, now);
            build
        }
    };
    if let Some(voted) = build.wait(builds.budget()).await {
        return vote_of(voted);
    }
    let progress = build.progress();
    Err(vote_building(
        asked,
        if progress.reading_for.is_some() {
            "deriving"
        } else {
            "queued"
        },
        progress.origin.word(),
        progress.queued_for,
        progress.reading_for,
        progress.counted,
        progress.owed,
    ))
}

/// A finished vote build's answer as the route takes it.
fn vote_of(voted: crate::calendar_of::build::Voted) -> Result<PeerCalendar, GapsAnswer> {
    voted.map(|vote| (*vote).clone()).map_err(|(status, body)| {
        (
            axum::http::StatusCode::from_u16(status)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )],
            body,
        )
    })
}

/// Run the vote build a request for `asked` leads, keep its vote when it may
/// be kept, and finish the build with it -- or with the admission refusal it
/// met, so nobody waits on a build that will never answer. Spawned, so the
/// build outlives a request that stopped waiting for it (DB-N19).
fn lead_vote(
    site: &Loaded,
    asked: &Addressed,
    build: &std::sync::Arc<crate::calendar_of::build::Build<crate::calendar_of::build::Voted>>,
    now: CensusStamps,
) {
    let key = vote_key(asked);
    let addressed = Addressed {
        vendor: asked.vendor,
        month: asked.month,
        timeframe: asked.timeframe,
        contract: asked.contract,
        exchange: asked.exchange.clone(),
        segment: asked.segment.clone(),
        symbol: asked.symbol.clone(),
    };
    let (site, build) = (std::sync::Arc::clone(site), std::sync::Arc::clone(build));
    drop(tokio::spawn(async move {
        let reading = {
            let (site, build, key) = (
                std::sync::Arc::clone(&site),
                std::sync::Arc::clone(&build),
                key.clone(),
            );
            crate::detail::run_calendar(move || {
                let _entered = build.enter(Some(now));
                let (keep, vote) = peer_vote_reading(&site, &addressed, census_now_stamped);
                let vote = std::sync::Arc::new(vote);
                if let Some(stamps) = keep {
                    let _kept_or_full =
                        site.calendars
                            .builds
                            .votes
                            .keep(key, stamps, std::sync::Arc::clone(&vote));
                }
                vote
            })
        };
        let voted = reading.await.map_err(|why| {
            let (status, _, body) = calendar_admission_refused(&why);
            (status.as_u16(), body)
        });
        site.calendars
            .builds
            .votes
            .flights
            .finish(&key, &build, voted);
    }));
}

/// The `202` a `calendar=peers` audit is answered while its vote is built:
/// what the build is doing, how far it has got, and when to ask again. No
/// minute is audited, so nothing here can be read as a verdict (DB-N19).
fn vote_building(
    asked: &Addressed,
    phase: &str,
    by: &str,
    since: std::time::Duration,
    reading: Option<std::time::Duration>,
    counted: u64,
    owed: u64,
) -> GapsAnswer {
    let millis =
        |elapsed: std::time::Duration| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let retry = crate::calendar_of::build::RETRY_AFTER_MS;
    let doing = match phase {
        "census" => {
            "the server is reading its census at start, which the vote is read from".to_owned()
        }
        "queued" => "it is queued".to_owned(),
        // NO PEER SERIES OWED YET: the walk of every feed's census entries
        // that finds them (about 0.3 s on the operator's store) is still
        // going, so "0 of 0" would read as a vote with nothing to count.
        _ if owed == 0 => "it is walking every feed's census for the peer series".to_owned(),
        _ => format!("it has derived {counted} of the {owed} peer series it reads so far"),
    };
    (
        axum::http::StatusCode::ACCEPTED,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "status": "building",
            "calendar": "peers",
            "feed": asked.vendor.as_str(),
            "exchange": asked.exchange,
            "segment": asked.segment,
            "symbol": asked.symbol,
            "phase": phase,
            "by": by,
            "since_ms": millis(since),
            "reading_ms": reading.map(millis),
            "series_counted": counted,
            "series_owed": owed,
            "retry_after_ms": retry,
            "reason": format!(
                "The peers' vote this audit counts against is being built off the request \
                 path: {doing}. No minute is audited until it is whole; ask again in {retry} ms, \
                 or ask for calendar=record, which needs no vote."
            ),
        })
        .to_string(),
    )
}

/// The `429` a `calendar=peers` audit is answered when every vote build the
/// map holds is queued or running.
fn votes_full() -> GapsAnswer {
    (
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "error": format!(
                "peer vote build not admitted: {} builds are queued or running, the most this \
                 server holds at once; retry, or ask for calendar=record",
                crate::calendar_of::build::MAX_BUILDS
            )
        })
        .to_string(),
    )
}

/// How many months one audit request will walk before it stops and says so.
///
/// Twenty years at the store's own granularity. The operator's store spans 121
/// months, so this is not a limit anyone reaches by accident — it exists so a
/// hand-typed `to=9999-12` cannot walk a hundred thousand months while holding
/// a request open.
const MAX_AUDIT_MONTHS: usize = 240;

/// [`MAX_AUDIT_MONTHS`] for a `1s` audit: twelve months.
///
/// A one-second index month is about 470,000 bars and is read whole, so a year
/// is about 5.6 million records in one request. Chosen, not sourced; the
/// answer says `truncated` when it bites, as the minute ceiling does.
const MAX_AUDIT_MONTHS_1S: usize = 12;

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
    /// Records `bars::page` could not read, or `None` when no record was
    /// read because the month's kept summary answered: rendered `null`,
    /// NOT `0`. A kept summary was built from a full read of the committed
    /// records when it was written, but damage to a record after that is not
    /// seen without reading the bars, and a `0` would claim a check this
    /// answer never made (R3-06 repair).
    unreadable: Option<usize>,
    /// Whether `MAX_GAPS` stopped this month's walk.
    truncated: bool,
    /// The runs, already rendered.
    runs: String,
    /// A `1s` month's own members, already rendered with a leading comma, or
    /// empty for a minute month.
    seconds: String,
    /// Where its summary came from (R3-06).
    summary: SummarySource,
    /// Why no kept summary was used, when one was not.
    fallback: Option<String>,
    /// Bar records read to answer it: zero when its summary was kept.
    records_read: u64,
}

impl AuditedMonth {
    /// This month as one JSON object.
    fn render(&self) -> String {
        format!(
            r#"{{"month":{},"expected":{},"held":{},"lost_minutes":{},"absent_minutes":{},"unmeasured_minutes":{},"truncated":{},"unreadable_records":{},"invalid_timestamps":{},"absent_file":{},"evidence_error":{},"summary":"{}","summary_fallback":{},"bar_records_read":{}{},"gaps":[{}]}}"#,
            render::json_string(&self.month.to_string()),
            self.expected,
            self.held,
            self.lost,
            self.absent,
            self.unmeasured,
            self.truncated,
            self.unreadable
                .map_or_else(|| "null".to_owned(), |n| n.to_string()),
            self.invalid_timestamps,
            self.absent_file
                .as_ref()
                .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
            self.evidence_error
                .as_ref()
                .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
            self.summary.word(),
            self.fallback
                .as_ref()
                .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
            self.records_read,
            self.seconds,
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

/// The word a run is rendered with.
///
/// `pull::gaps` has four reasons and no fifth. Three more are this route's,
/// and none of them is a loss: a minute inside a recorded halt's window is
/// `halted` (sourced) or `halt-suspected` (UNVERIFIED, D-3320) rather than
/// `outside-window`; a minute with no print in a trade-driven series (a `1s`
/// share or option, D-3336 §2) is `no-print` rather than `vendor-hole`; and a
/// minute after a contract's expiry day is `expired`; and a missing minute
/// on a day past the last one whose import of the segment's kind closed, or
/// on a day whose import began and never closed, is `not-imported` — the
/// import has not written it, which says nothing about the vendor (R3-05,
/// CMP-04, SC-CLOSE-2).
fn run_word(
    gap: &pull::gaps::Gap,
    trade_driven: bool,
    expiry: Option<i64>,
    reach: &super::feeds_route::ImportReach,
) -> &'static str {
    if expiry.is_some_and(|last| gap.day > last) {
        return "expired";
    }
    // A DAY THAT HAS NOT HAPPENED YET owes nothing: neither a loss nor "not
    // imported" (R3-05 repair 3).
    if gap.reason == pull::gaps::Reason::VendorHole && reach.not_yet(gap.day) {
        return NOT_YET_WORD;
    }
    if gap.reason == pull::gaps::Reason::VendorHole && reach.not_imported(gap.day) {
        return NOT_IMPORTED_WORD;
    }
    if gap.reason == pull::gaps::Reason::OutsideWindow
        && let Some(halt) = pull::calendar::halt_on(gap.day)
        && halt.halted.holds(gap.from)
        && halt.halted.holds(gap.to)
    {
        return if halt.evidence.is_suspected() {
            "halt-suspected"
        } else {
            "halted"
        };
    }
    if trade_driven && gap.reason == pull::gaps::Reason::VendorHole {
        return "no-print";
    }
    gap.reason.word()
}

/// The run word for a day the import has not reached (R3-05, CMP-04).
const NOT_IMPORTED_WORD: &str = "not-imported";

/// The word for a missing minute on a day after today (IST): the session
/// has not happened, so the minute is neither owed nor lost (R3-05 repair 3).
const NOT_YET_WORD: &str = "not-yet";

/// The last session day of a contract: the expiry date its segment opens
/// with (`YYYY-MM-DD-…`), or `None` for a spot series or a contract whose
/// text does not start with a date.
fn expiry_of(contract: Option<&brutex_core::instrument::Contract>) -> Option<i64> {
    let text = contract?.as_str().get(..10)?;
    let mut parts = text.split('-');
    let year = parts.next()?.parse::<u16>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    pull::session::Day::new(year, month, day)
        .ok()
        .map(|day| i64::from(day.days_from_epoch()))
}

/// Audit one month of one series.
///
/// # The range comes from the MONTH, not from the bars
///
/// Deriving it from the first and last stored bar is the mistake that makes
/// this agree with itself and disagree with the store: a month whose first four
/// trading days never landed would report a clean interior and a perfect score,
/// because the missing days would fall outside a range they themselves defined.
///
/// # A one-second month (D-3336 §4)
///
/// Its seconds are folded to minutes with [`pull::fold::fold_from_bars`] and
/// the minutes are classified exactly as a minute month's are. A spot index
/// owes every second of a session (22,500 a regular day), so its seconds are
/// audited too ([`second_audit`]). A share or an option is trade-driven: it
/// owes no count, a minute with no print is `no-print`, never a loss, and the
/// month says how many of its sessions it holds a print on.
#[expect(
    clippy::too_many_lines,
    reason = "one open and one classification for both rungs, the fold and the two \
              one-second verdicts beside them; split, the rows would be read twice"
)]
fn audit_one(
    site: &Loaded,
    asked: &Addressed,
    month: store::path::YearMonth,
    calendar: Option<&pull::calendar::Calendar>,
    cash_schedule: Option<&pull::cash_auction::Schedule>,
    min_run: u32,
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
        unreadable: Some(0),
        truncated: false,
        runs: String::new(),
        seconds: String::new(),
        summary: SummarySource::AbsentFile,
        fallback: None,
        records_read: 0,
    };

    let Ok(first_day) = pull::session::Day::new(month.year(), month.month(), 1) else {
        return empty(Some(format!(
            "{month} is a month this build's calendar cannot name a first day \
             for, so there is no range to audit against"
        )));
    };
    let first = i64::from(first_day.days_from_epoch());
    let last = i64::from(first_day.end_of_month().days_from_epoch());
    let one_second = asked.timeframe == store::path::Timeframe::SECOND_1;
    let trade_driven = one_second && asked.segment != "INDEX";
    let expiry = expiry_of(asked.contract.as_ref());
    // HOW FAR THE IMPORT HAS REACHED for this feed's segment, and the days
    // of this month whose import began and never closed, from the GDFL
    // import journal: a cached probe, O(1) while the journal does not move
    // (R3-05).
    let reach = super::feeds_route::imported_to(
        &site.store_root,
        asked.vendor,
        &asked.segment,
        (first, last),
    );

    // ONE SUMMARY A MONTH, AND NO BAR, when the month keeps one that
    // describes its committed header (R3-06, D-4980, DB-A30); otherwise its
    // bars are read and summarised by the same library call, and the answer
    // names why. Every fact below is read off the summary either way, so the
    // kept and the read answer are one computation.
    let needs_seconds = one_second && !trade_driven;
    let input = month_input(site, asked, month, needs_seconds);
    let held = &input.summary;
    // ONE SECOND FOLDS TO ONE MINUTE THROUGH THE ONE FOLD AUTHORITY: the
    // minutes a one-second month holds are its summary's minute index, which
    // is what `pull::fold` makes of rows in order. Rows the fold refuses (a
    // stamp before the one before it, a negative volume) are input this audit
    // cannot certify: it says why, in the fold's own words, and claims nothing.
    let fold_error = input.fold_error.clone();
    let stamps: Vec<i64> = if fold_error.is_some() {
        Vec::new()
    } else {
        held.minute_stamps()
    };
    // A MARKET SESSION AND AN INDEX-PUBLICATION WINDOW ARE NOT THE SAME FACT.
    // On 2021-02-24 SEBI proves 220 normal-market minutes, but also records
    // NIFTY computation unavailable during part of the still-trading morning
    // and gives no common interval for every stored index. The index-specific
    // door therefore refuses to turn that day into invented vendor holes;
    // CASH and derivative series keep the verified exchange-session calendar.
    let mut ledger = if fold_error.is_some() {
        pull::gaps::Ledger::default()
    } else if !one_second && held.counts.invalid > 0 {
        // OFF THE MINUTE GRID OR NOT STRICTLY INCREASING: what `pull::gaps`
        // refuses to classify, counted from the summary rather than re-read.
        pull::gaps::Ledger {
            held: u32::try_from(held.counts.folded).unwrap_or(u32::MAX),
            invalid_timestamps: held.counts.invalid,
            ..pull::gaps::Ledger::default()
        }
    } else if trade_driven && asked.segment == "CASH" {
        // NOTHING IS OWED, so the dated cash schedule decides nothing: a
        // minute of the exchange session with no print is `no-print`.
        pull::gaps::classify_against(&stamps, first, last, calendar)
    } else if asked.exchange == "NSE" && asked.segment == "CASH" && asked.contract.is_none() {
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
    // `held` IS WHAT A FULL READ STATES: a minute month's rows, a one-second
    // month's folded minutes, a row outside the month included, though the
    // minute index the classifier walked holds only the month's own days.
    if fold_error.is_none() {
        let held_rows = if one_second {
            u64::from(held.counts.held_minutes)
        } else {
            held.counts.folded
        };
        ledger.held = u32::try_from(held_rows).unwrap_or(u32::MAX);
    }

    // RUNS, NOT MINUTES, and the reason travels with each one. 1,176 of 1,204
    // measured absences were four contiguous events; a per-minute list is the
    // same facts 43× longer.
    let words: Vec<&'static str> = ledger
        .gaps
        .iter()
        .map(|gap| run_word(gap, trade_driven, expiry, &reach))
        .collect();
    let runs = ledger
        .gaps
        .iter()
        .zip(&words)
        .map(|(gap, word)| {
            format!(
                r#"{{"day":{},"from":{},"to":{},"minutes":{},"reason":{}}}"#,
                gap.day,
                gap.from,
                gap.to,
                gap.minutes(),
                render::json_string(word)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let minutes_of = |word: &str, lost_only: bool| -> u32 {
        ledger
            .gaps
            .iter()
            .zip(&words)
            .filter(|(gap, w)| {
                **w == word && (!lost_only || gap.reason == pull::gaps::Reason::VendorHole)
            })
            .map(|(gap, _)| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    };
    // A CONTRACT OWES NOTHING AFTER ITS EXPIRY DAY: a minute the calendar owes
    // there, and the contract cannot print, is `expired`, neither owed nor
    // lost. Only what `pull::gaps` counted as a loss is taken back out, from
    // the loss and from what was owed; a closed day renamed `expired` was
    // never either.
    let expired_lost = minutes_of("expired", true);
    // A DAY THE IMPORT HAS NOT REACHED IS NOT A LOSS EITHER (CMP-04): its
    // minutes leave the loss and the owed count exactly as `expired` does,
    // and each run still names it.
    let not_imported = minutes_of(NOT_IMPORTED_WORD, true);
    // NOR IS A DAY THAT HAS NOT HAPPENED YET (R3-05 repair 3): out of the
    // loss, out of what was owed, and out of `not_imported_minutes` too.
    let not_yet = minutes_of(NOT_YET_WORD, true);
    let mut lost_minutes = ledger
        .lost_minutes()
        .saturating_sub(expired_lost)
        .saturating_sub(not_imported)
        .saturating_sub(not_yet);
    ledger.expected = ledger
        .expected
        .saturating_sub(expired_lost)
        .saturating_sub(not_imported)
        .saturating_sub(not_yet);
    let mut members = halts_member(&stamps, first, last);
    // THE DAYS THE IMPORT HAS NOT REACHED, COUNTED APART (CMP-04): neither
    // lost nor owed, and shown beside the day the census says the import
    // reached for this feed's segment — `null` when the census cannot say, in
    // which case nothing was renamed.
    members.push_str(r#","not_imported_minutes":"#);
    members.push_str(&not_imported.to_string());
    members.push_str(r#","not_yet_minutes":"#);
    members.push_str(&not_yet.to_string());
    members.push_str(r#","imported_to":"#);
    members.push_str(
        &reach
            .to
            .map_or_else(|| "null".to_owned(), |day| render::json_string(&iso(day))),
    );
    members.push_str(r#","imported_to_from":"#);
    members.push_str(&render::json_string(&reach.from));
    // THE DAYS WHOSE IMPORT BEGAN AND NEVER CLOSED, inside the reach: not
    // imported, never lost.
    members.push_str(r#","import_open_days":["#);
    members.push_str(
        &reach
            .open
            .iter()
            .map(|day| render::json_string(&iso(*day)))
            .collect::<Vec<_>>()
            .join(","),
    );
    members.push(']');
    if one_second {
        let no_print = minutes_of("no-print", false);
        // THE SESSION MINUTES AFTER EXPIRY, not every minute of those days:
        // a closed weekend after expiry was never owed either way.
        let expired = minutes_of("expired", true);
        if trade_driven {
            // A TRADE-DRIVEN SERIES OWES NO COUNT (D-3336 §2): nothing it lacks
            // is a loss, and `expected` is not a denominator for it.
            lost_minutes = 0;
            ledger.expected = 0;
        }
        members += &one_second_members(
            held,
            first,
            last,
            calendar,
            trade_driven,
            expiry,
            min_run,
            fold_error.as_deref(),
            (no_print, expired),
            &reach,
        );
    }

    AuditedMonth {
        month,
        expected: ledger.expected,
        held: ledger.held,
        invalid_timestamps: ledger.invalid_timestamps,
        lost: lost_minutes,
        absent: ledger.absent_minutes(),
        unmeasured: ledger.unmeasured_minutes(),
        // A FAULTY RECORD IS NOT SILENTLY SKIPPED, and here it matters more
        // than on the drawing route: a bar `page` could not read is a bar this
        // audit did not see, so it is scored as a HOLE. Reported beside the
        // verdict so "the file is damaged" cannot be read as "the vendor is
        // missing minutes" — opposite faults wanting opposite fixes.
        absent_file: input.absent_file,
        evidence_error: None,
        // RECORDS, NOT SENTENCES: `page` coalesces a run of refusals into
        // one sentence (DB-B75) and counts every record.
        unreadable: input
            .unreadable
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX)),
        truncated: ledger.truncated,
        runs,
        seconds: members,
        summary: input.source,
        fallback: input.fallback,
        records_read: input.records_read,
    }
}

/// Where a month's summary came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummarySource {
    /// Kept beside the month and describing its committed header: no bar read.
    Kept,
    /// Built from the month's bars, read whole, because none was usable.
    Built,
    /// The month has no file; the summary is empty.
    AbsentFile,
}

impl SummarySource {
    const fn word(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::Built => "built",
            Self::AbsentFile => "absent_file",
        }
    }
}

/// One month's input to the audit: its summary, and how it was had.
struct MonthInput {
    summary: store::month_summary::MonthSummary,
    source: SummarySource,
    /// Why no kept summary was used, when one was not.
    fallback: Option<String>,
    /// Bar records read to answer: zero when the summary was kept.
    records_read: u64,
    /// Records `bars::page` could not read; `None` when no record was read.
    unreadable: Option<u64>,
    absent_file: Option<String>,
    /// The fold's refusal of a one-second month's rows, in its own words.
    fold_error: Option<String>,
}

/// The month's summary: kept and usable, or built from its bars with the
/// reason named (R3-06, D-4980).
///
/// A kept summary is used when it describes the committed header (one
/// comparison), its stamps are ones the audit can certify, and it keeps the
/// seconds a spot index's one-second audit needs. Anything else reads the
/// bars: a month under 1,024 bars (at most 1,023 records, D-4933), a month
/// whose summary is absent, torn, stale or foreign, and a month whose input
/// the audit refuses, so the refusal is stated from the rows themselves.
fn month_input(
    site: &Loaded,
    asked: &Addressed,
    month: store::path::YearMonth,
    needs_seconds: bool,
) -> MonthInput {
    let file = match asked.open(site, month) {
        Ok(file) => file,
        Err(why) => {
            return MonthInput {
                summary: store::month_summary::MonthSummary::new(0, asked.timeframe.secs(), month),
                source: SummarySource::AbsentFile,
                fallback: None,
                records_read: 0,
                unreadable: Some(0),
                absent_file: Some(why),
                fold_error: None,
            };
        }
    };
    let header = file.header();
    let one_second = asked.timeframe == store::path::Timeframe::SECOND_1;
    let fallback = match store::month_summary::read_beside(file.path(), &header, month) {
        // INPUT THE FOLD MAY REFUSE: a stamp before the one before it, a
        // negative volume, a volume summed past `i64::MAX` (any minute's
        // bucket overflowing makes the month's saturate) or a row outside the
        // month (where an anchor can overflow). The refusal, if any, is the
        // fold's, in its own words, read from the rows.
        Ok(kept)
            if one_second
                && (kept.counts.decreasing > 0
                    || kept.counts.negative_volume > 0
                    || kept.counts.outside_month > 0
                    || kept.extremes.volume == i64::MAX) =>
        {
            "the month holds rows the fold may refuse; its answer is read from the bars".to_owned()
        }
        Ok(kept) if needs_seconds && !kept.tracks_seconds() => {
            "the kept summary holds no seconds index (fewer than half the seconds it spans \
             are held, D-4980); an index's seconds are read from the bars"
                .to_owned()
        }
        Ok(kept) => {
            return MonthInput {
                summary: kept,
                source: SummarySource::Kept,
                fallback: None,
                records_read: 0,
                // NOT READ, so not counted: see `AuditedMonth::unreadable`.
                unreadable: None,
                absent_file: None,
                fold_error: None,
            };
        }
        Err(why) => why.to_string(),
    };
    // THE WHOLE MONTH, by index. `page` reads at a fixed stride, so this is
    // `n_valid` seeks of known length and nothing scans.
    let n = usize::try_from(header.n_valid).unwrap_or(usize::MAX);
    let (rows, faults) = bars::page(&file, 0, n);
    let fold_error = one_second
        .then(|| {
            pull::fold::fold_from_bars(
                &rows,
                pull::fold::Bucket::MINUTE,
                pull::fold::Bucket::SECOND,
            )
            .err()
            .map(|why| format!("{why:?}"))
        })
        .flatten();
    MonthInput {
        summary: store::month_summary::MonthSummary::build(&header, month, &rows),
        source: SummarySource::Built,
        fallback: Some(fallback),
        // EVERY RECORD THE AUDIT READ, the ones the file refused included:
        // the cost of the slow path is the reads it made, not the rows it kept.
        records_read: u64::try_from(rows.len())
            .unwrap_or(u64::MAX)
            .saturating_add(faults.records()),
        unreadable: Some(faults.records()),
        absent_file: None,
        fold_error,
    }
}

/// Every recorded halt of the month and the minutes they take out of what is
/// owed, rendered with a leading comma.
///
/// A halted minute is neither owed nor missing (D-3336 §1), so a halt whose
/// minutes the store does hold leaves no run behind it, and a reader could not
/// tell the month crossed one. The month counts the minutes apart, a
/// suspected halt's (`halt_suspected_minutes`, UNVERIFIED, D-3320) separately
/// from a sourced one's (`halted_minutes`), and names each halt with its
/// window, whether it is suspected, and how many of its minutes the series
/// holds a bar in: a straggling print is a bar, labelled, never removed.
fn halts_member(stamps: &[i64], first: i64, last: i64) -> String {
    let mut suspected_minutes = 0_u32;
    let mut sourced_minutes = 0_u32;
    let halts: Vec<String> = pull::calendar::HALTS
        .iter()
        .filter(|halt| (first..=last).contains(&halt.day))
        .map(|halt| {
            let minutes = u32::from(halt.halted.to.saturating_sub(halt.halted.from)) + 1;
            if halt.evidence.is_suspected() {
                suspected_minutes = suspected_minutes.saturating_add(minutes);
            } else {
                sourced_minutes = sourced_minutes.saturating_add(minutes);
            }
            let inside = stamps
                .iter()
                .filter(|stamp| {
                    let (day, second) = ist_second(**stamp);
                    day == halt.day
                        && u16::try_from(second / 60).is_ok_and(|minute| halt.halted.holds(minute))
                })
                .count();
            format!(
                r#"{{"day":{},"date":"{}","from":{},"to":{},"minutes":{minutes},"suspected":{},"held_minutes_inside":{inside}}}"#,
                halt.day,
                iso(halt.day),
                halt.halted.from,
                halt.halted.to,
                halt.evidence.is_suspected(),
            )
        })
        .collect();
    format!(
        r#","halt_suspected_minutes":{suspected_minutes},"halted_minutes":{sourced_minutes},"halts":[{}]"#,
        halts.join(",")
    )
}

/// A `1s` month's own members, rendered with a leading comma.
#[expect(
    clippy::too_many_arguments,
    reason = "the month's rows and days, its calendar, and the four facts that decide \
              what a second owes: trade-driven, expiry, the listing bound, a fold refusal, \
              and how far the import has reached"
)]
fn one_second_members(
    held: &store::month_summary::MonthSummary,
    first: i64,
    last: i64,
    calendar: Option<&pull::calendar::Calendar>,
    trade_driven: bool,
    expiry: Option<i64>,
    min_run: u32,
    fold_error: Option<&str>,
    (no_print, expired): (u32, u32),
    reach: &super::feeds_route::ImportReach,
) -> String {
    let fold_error = fold_error.map_or_else(|| "null".to_owned(), render::json_string);
    if trade_driven {
        let days = trade_days(held, first, last, calendar, expiry);
        let without = days
            .without
            .iter()
            .take(MAX_LISTED_DAYS)
            .map(|day| format!(r#"{{"day":{day},"date":"{}"}}"#, iso(*day)))
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            r#","trade_driven":true,"fold_error":{fold_error},"no_print_minutes":{no_print},"expired_minutes":{expired},"sessions":{},"days_held":{},"days_without_print":[{without}],"days_without_print_count":{},"expiry":{}"#,
            days.sessions,
            days.held,
            days.without.len(),
            expiry.map_or_else(|| "null".to_owned(), |day| format!(r#""{}""#, iso(day))),
        );
    }
    let mut audit = second_audit(held, first, last, calendar, min_run);
    // A RUN ON A DAY THE IMPORT HAS NOT REACHED is named so, never a vendor
    // hole (R3-05, CMP-04).
    for run in &mut audit.runs {
        if reach.not_yet(run.day) {
            run.reason = NOT_YET_WORD;
        } else if reach.not_imported(run.day) {
            run.reason = NOT_IMPORTED_WORD;
        }
    }
    let listed = audit
        .runs
        .iter()
        .map(|run| {
            format!(
                r#"{{"day":{},"date":"{}","from":"{}","to":"{}","from_second":{},"to_second":{},"seconds":{},"reason":"{}"}}"#,
                run.day,
                iso(run.day),
                clock_secs(run.from),
                clock_secs(run.to),
                run.from,
                run.to,
                run.to.saturating_sub(run.from).saturating_add(1),
                run.reason,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let days = audit
        .days
        .iter()
        .map(|day| {
            format!(
                r#"{{"day":{},"date":"{}","expected_seconds":{},"held_seconds":{},"missing_seconds":{},"longest_run":{}}}"#,
                day.day,
                iso(day.day),
                day.expected,
                day.held,
                day.missing,
                day.longest.map_or_else(
                    || "null".to_owned(),
                    |(from, length)| format!(r#"{{"from":"{}","seconds":{length}}}"#, clock_secs(from))
                ),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#","trade_driven":false,"fold_error":{fold_error},"seconds":{{"expected":{},"held":{},"missing":{},"outside":{},"runs":{},"runs_listed":{},"min_run":{min_run},"truncated":{}}},"second_runs":[{listed}],"days":[{days}]"#,
        audit.expected,
        audit.held,
        audit.missing,
        audit.outside,
        audit.run_count,
        audit.runs.len(),
        audit.truncated,
    )
}

/// The longest list of days one month names; its count always travels beside it.
const MAX_LISTED_DAYS: usize = 31;

/// `epoch_day` as `YYYY-MM-DD`.
fn iso(epoch_day: i64) -> String {
    crate::calendar_of::record::iso(epoch_day)
}

/// A second of the day as `HH:MM:SS`.
fn clock_secs(second: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        second / 3_600,
        second % 3_600 / 60,
        second % 60
    )
}

/// The IST day and second-of-day of a micros-since-epoch stamp.
fn ist_second(ts_micros: i64) -> (i64, u32) {
    let secs = ts_micros.div_euclid(1_000_000).saturating_add(19_800);
    (
        secs.div_euclid(86_400),
        u32::try_from(secs.rem_euclid(86_400)).unwrap_or(u32::MAX),
    )
}

/// What a trade-driven month holds against the sessions it could have traded.
struct TradeDays {
    /// Sessions of the month inside the contract's life.
    sessions: u32,
    /// Of those, the sessions with at least one print.
    held: u32,
    /// The sessions with none, in order.
    without: Vec<i64>,
}

/// Sessions held and not held by a trade-driven month (D-3336 §2): a session
/// is a day `calendar` sizes, on or before `expiry`.
fn trade_days(
    held: &store::month_summary::MonthSummary,
    first: i64,
    last: i64,
    calendar: Option<&pull::calendar::Calendar>,
    expiry: Option<i64>,
) -> TradeDays {
    let mut days = TradeDays {
        sessions: 0,
        held: 0,
        without: Vec::new(),
    };
    for day in first..=last.min(expiry.unwrap_or(last)) {
        let open = calendar.map_or_else(
            || pull::calendar::exchange_kind_of(day),
            |calendar| calendar.kind_of(day),
        );
        if !matches!(open, pull::calendar::DayKind::Open(_)) {
            continue;
        }
        days.sessions = days.sessions.saturating_add(1);
        if held.bars_on(day) > 0 {
            days.held = days.held.saturating_add(1);
        } else {
            days.without.push(day);
        }
    }
    days
}

/// One run of missing seconds.
struct SecondRun {
    day: i64,
    from: u32,
    to: u32,
    reason: &'static str,
}

/// One day of a second-level audit.
struct SecondDay {
    day: i64,
    expected: u64,
    held: u64,
    missing: u64,
    /// The longest run of missing seconds: its first second and its length.
    longest: Option<(u32, u32)>,
}

/// A spot index's seconds against what the calendar owes.
#[derive(Default)]
struct SecondAudit {
    expected: u64,
    held: u64,
    missing: u64,
    /// Seconds held that no session owes: before the open, after the close,
    /// inside a halt, on a closed or unmeasured day. Counted, never removed.
    outside: u64,
    /// Every run of missing seconds, listed or not.
    run_count: u64,
    /// The runs at least `min_run` long, at most [`pull::gaps::MAX_GAPS`].
    runs: Vec<SecondRun>,
    truncated: bool,
    days: Vec<SecondDay>,
}

/// A spot index's one-second month, second by second (D-3336 §1, §4).
///
/// A second is owed when its minute is a session minute of `calendar` (the
/// record carves a halt's whole minutes out) and it is not one of the seconds
/// the index spends in a recorded halt ([`pull::calendar::Halt::seconds`]):
/// 22,500 on a regular session. 2021-02-24 owes none: the index's publication
/// window that day is unproved (D-0420), as the minute door says. Every
/// missing second is counted; runs at least `min_run` seconds long are listed,
/// and every day names its longest run whatever its length.
///
/// The seconds held come off the month summary's seconds index, distinct and
/// in order, so no bar is read (R3-06): one pass over each day's owed
/// seconds, 86,400 a day, whatever the bars per month.
fn second_audit(
    summary: &store::month_summary::MonthSummary,
    first: i64,
    last: i64,
    calendar: Option<&pull::calendar::Calendar>,
    min_run: u32,
) -> SecondAudit {
    let mut audit = SecondAudit::default();
    for day in first..=last {
        // A SUMMARY WITHOUT SECONDS NEVER REACHES HERE: `month_input` reads
        // the bars for a spot index whose kept summary holds none.
        let held: Vec<u32> = summary.seconds_on(day).unwrap_or_default();
        let kind = calendar.map_or_else(
            || pull::calendar::exchange_kind_of(day),
            |calendar| calendar.kind_of(day),
        );
        let session = match kind {
            pull::calendar::DayKind::Open(session) if day != pull::calendar::SYSTEMS_OUTAGE_DAY => {
                session
            }
            _ => {
                audit.outside = audit.outside.saturating_add(held.len() as u64);
                continue;
            }
        };
        let halt =
            pull::calendar::halt_on(day).map(|halt| halt.seconds(pull::vendor::Venue::NseIndex));
        let owes = |second: u32| {
            u16::try_from(second / 60).is_ok_and(|minute| session.expects(minute))
                && !halt.is_some_and(|h| (h.from..h.resume).contains(&second))
        };
        let mut one = SecondDay {
            day,
            expected: 0,
            held: 0,
            missing: 0,
            longest: None,
        };
        let mut cursor = held.iter().copied().peekable();
        let mut run: Option<(u32, u32)> = None;
        for second in 0..86_400_u32 {
            let present = cursor.next_if_eq(&second).is_some();
            if !owes(second) {
                if present {
                    audit.outside = audit.outside.saturating_add(1);
                }
                close_run(&mut audit, &mut one, &mut run, min_run);
                continue;
            }
            one.expected = one.expected.saturating_add(1);
            if present {
                one.held = one.held.saturating_add(1);
                close_run(&mut audit, &mut one, &mut run, min_run);
            } else {
                one.missing = one.missing.saturating_add(1);
                run = Some(run.map_or((second, second), |(from, _)| (from, second)));
            }
        }
        close_run(&mut audit, &mut one, &mut run, min_run);
        audit.expected = audit.expected.saturating_add(one.expected);
        audit.held = audit.held.saturating_add(one.held);
        audit.missing = audit.missing.saturating_add(one.missing);
        audit.days.push(one);
    }
    // ROWS OUTSIDE THE MONTH'S DAYS are not this month's, and are counted as
    // held outside any owed second rather than dropped.
    audit.outside = audit
        .outside
        .saturating_add(u64::from(summary.counts.outside_month));
    audit
}

/// Close the open run of missing seconds, if any: count it, keep the day's
/// longest, and list it when it is at least `min_run` long and there is room.
fn close_run(
    audit: &mut SecondAudit,
    day: &mut SecondDay,
    run: &mut Option<(u32, u32)>,
    min_run: u32,
) {
    let Some((from, to)) = run.take() else {
        return;
    };
    let length = to.saturating_sub(from).saturating_add(1);
    audit.run_count = audit.run_count.saturating_add(1);
    if day.longest.is_none_or(|(_, longest)| length > longest) {
        day.longest = Some((from, length));
    }
    if length < min_run {
        return;
    }
    if audit.runs.len() >= pull::gaps::MAX_GAPS {
        audit.truncated = true;
        return;
    }
    audit.runs.push(SecondRun {
        day: day.day,
        from,
        to,
        reason: "vendor-hole",
    });
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
        let absent = audit_one(
            &site,
            &asked,
            asked.month,
            Some(&calendar),
            None,
            MIN_RUN_DEFAULT,
        );
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
        let empty = audit_one(
            &site,
            &asked,
            asked.month,
            Some(&calendar),
            None,
            MIN_RUN_DEFAULT,
        );
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
            let audit = audit_one(
                &site,
                &asked,
                asked.month,
                Some(&calendar),
                None,
                MIN_RUN_DEFAULT,
            );
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
            let audit = audit_one(&site, &asked, asked.month, None, None, MIN_RUN_DEFAULT);
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

        let verdict = audit_one(&site, &asked, month, Some(&calendar), None, MIN_RUN_DEFAULT);
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
            three.contains(r#""source":"record""#) && three.contains(r#""voted_by":[]"#),
            "the calendar of record answers (D-3336), and the response names it \
             rather than leaving a number un-attributable: {three}"
        );
        assert!(
            three.contains(r#""lost_minutes":8249"#),
            "one bar stored against a month the calendar owes 8,250 — the table \
             is independent of the bars, so it can still say that: {three}"
        );
        // ONE MONTH SPELLING (app-7, AF-04, D-4962). This asserted that
        // `to=2024-1` answered as January — the lenient parse the refusal
        // sentence itself denied. `YYYY-MM` is the only spelling every route
        // reads, so an unpadded month is refused with that sentence.
        let (short_code, _, short) = ask("&to=2024-1").await;
        assert_eq!(
            short_code,
            axum::http::StatusCode::BAD_REQUEST,
            "an unpadded month is not a YYYY-MM month: {short}"
        );
        assert!(
            short.contains(r#"\"2024-1\" is not a YYYY-MM month for \"to\""#),
            "the refusal names the spelling and the field: {short}"
        );
    }

    /// Write `rows` for one series' month at `rung`, with `contract` below the
    /// symbol when given.
    #[expect(
        clippy::too_many_arguments,
        reason = "one bar file's address and its rows"
    )]
    fn write_rows(
        root: &std::path::Path,
        vendor: Vendor,
        segment: &str,
        symbol: &str,
        contract: Option<&str>,
        rung: store::path::Timeframe,
        month: store::path::YearMonth,
        stamps: &[i64],
    ) {
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor,
            exchange: "NSE",
            segment,
            symbol,
            contract: contract
                .map(|text| brutex_core::instrument::Contract::parse(text).expect("a contract")),
            timeframe: rung,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX))
            .expect("low 32 bits");
        let mut file = store::file::BarFile::open_or_create(root, path, id).expect("a file");
        let rows: Vec<store::format::Bar> = stamps
            .iter()
            .map(|&ts_micros| store::format::Bar {
                ts_micros,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: i64::MIN,
            })
            .collect();
        file.append(&rows).expect("ordered rows");
    }

    /// Micros since the epoch of `second` seconds past IST midnight on `day`.
    fn second_of(day: i64, second: i64) -> i64 {
        (day * 86_400 - 19_800 + second) * 1_000_000
    }

    /// `/gaps.json` with `query`, parsed.
    async fn gaps(
        site: &std::sync::Arc<Site>,
        query: &str,
    ) -> (axum::http::StatusCode, serde_json::Value) {
        let uri: axum::http::Uri = format!("/gaps.json?{query}").parse().expect("a uri");
        let (code, _, body) =
            gaps_json(axum::extract::State(std::sync::Arc::clone(site)), uri).await;
        (code, serde_json::from_str(&body).expect("JSON"))
    }

    /// **A ONE-SECOND INDEX MONTH IS FOLDED TO MINUTES AND AUDITED SECOND BY
    /// SECOND, AND THE 34-SECOND HOLE IS NAMED.** DB-073, D-3336 §4: `1s` was
    /// refused as "coarser" and no GDFL series could be audited.
    ///
    /// 2018-09-03 holds every second of its session but a 34-second stretch
    /// from 12:21:40 and an isolated missing second at 10:00:00; the month's
    /// other sessions hold nothing. Every minute of the 3rd still has a print,
    /// so it loses no minute; its seconds owe 22,500 and miss 35; the long run
    /// is listed and the short one only counted; the other sessions are lost
    /// minutes, whole.
    #[tokio::test]
    async fn a_one_second_index_month_is_folded_to_minutes_and_audited_by_the_second() {
        let root = store_root("gaps-one-second-index");
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let day = i64::from(day(2018, 9, 3).days_from_epoch());
        let hole = (12 * 3_600 + 21 * 60 + 40)..(12 * 3_600 + 21 * 60 + 74);
        let lone = 10 * 3_600;
        let stamps: Vec<i64> = (33_300..55_800)
            .filter(|second| !hole.contains(second) && *second != lone)
            .map(|second| second_of(day, second))
            .collect();
        write_rows(
            &root,
            Vendor::Gdfl,
            "INDEX",
            "NIFTY",
            None,
            store::path::Timeframe::SECOND_1,
            month,
            &stamps,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-one-second-index", None, None),
            &root,
        ));
        let (code, body) = gaps(
            &site,
            "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09",
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert_eq!(body["timeframe"], "1s");
        assert_eq!(body["calendar"]["source"], "record");
        let audited = &body["month"][0];
        assert_eq!(audited["trade_driven"], false);
        let first = &audited["days"][0];
        assert_eq!(first["date"], "2018-09-03");
        assert_eq!(first["expected_seconds"], 22_500);
        assert_eq!(first["held_seconds"], 22_465);
        assert_eq!(first["missing_seconds"], 35);
        assert_eq!(first["longest_run"]["from"], "12:21:40");
        assert_eq!(first["longest_run"]["seconds"], 34);
        assert_eq!(
            audited["seconds"]["runs"], 19,
            "the two runs of the 3rd and 17 whole days"
        );
        assert_eq!(audited["second_runs"][0]["from"], "12:21:40");
        assert_eq!(audited["second_runs"][0]["to"], "12:22:13");
        assert_eq!(audited["second_runs"][0]["seconds"], 34);
        assert_eq!(audited["seconds"]["min_run"], 10);
        let sessions = 18_u64;
        assert_eq!(audited["seconds"]["expected"], sessions * 22_500);
        assert_eq!(audited["expected"], sessions * 375);
        assert_eq!(
            audited["lost_minutes"],
            (sessions - 1) * 375,
            "the 3rd loses no minute; every other session is lost whole"
        );
        // EVERY RUN IS LISTED AT `min_run=1`, AND A BAD VALUE IS REFUSED.
        let (_, all) = gaps(
            &site,
            "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09&min_run=1",
        )
        .await;
        assert_eq!(all["month"][0]["second_runs"][0]["from"], "10:00:00");
        for bad in ["0", "3601", "ten"] {
            let (code, refused) = gaps(
                &site,
                &format!("feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09&min_run={bad}"),
            )
            .await;
            assert_eq!(
                code,
                axum::http::StatusCode::BAD_REQUEST,
                "{bad}: {refused}"
            );
        }
    }

    /// **A ONE-SECOND OPTION IS TRADE-DRIVEN: IT OWES NO COUNT, A MINUTE WITH
    /// NO PRINT IS `no-print`, AND IT IS MEASURED BY THE SESSIONS IT HOLDS A
    /// PRINT ON UP TO ITS EXPIRY.** D-3336 §2.
    #[tokio::test]
    async fn a_one_second_option_is_trade_driven_and_measured_by_days_held() {
        let root = store_root("gaps-one-second-option");
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let (third, fourth) = (
            i64::from(day(2018, 9, 3).days_from_epoch()),
            i64::from(day(2018, 9, 4).days_from_epoch()),
        );
        let stamps: Vec<i64> = [third, fourth]
            .iter()
            .flat_map(|d| [second_of(*d, 33_300), second_of(*d, 40_000)])
            .collect();
        write_rows(
            &root,
            Vendor::Gdfl,
            "FNO",
            "NIFTY",
            Some("2018-09-27-1100000-CE"),
            store::path::Timeframe::SECOND_1,
            month,
            &stamps,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-one-second-option", None, None),
            &root,
        ));
        let (code, body) = gaps(
            &site,
            "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2018-09-27-1100000-CE&timeframe=1s&month=2018-09",
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        let audited = &body["month"][0];
        assert_eq!(audited["trade_driven"], true, "{audited}");
        assert_eq!(audited["expected"], 0, "no owed count");
        assert_eq!(audited["lost_minutes"], 0, "nothing it lacks is a loss");
        assert_eq!(audited["held"], 4, "four minutes with a print");
        assert_eq!(
            audited["sessions"], 17,
            "the sessions of September up to the 27th"
        );
        assert_eq!(audited["days_held"], 2);
        assert_eq!(audited["days_without_print_count"], 15);
        assert_eq!(audited["expiry"], "2018-09-27");
        assert_eq!(
            audited["expired_minutes"], 375,
            "the 28th's session minutes, not every minute of the 28th to the 30th"
        );
        let reasons: std::collections::BTreeSet<&str> = audited["gaps"]
            .as_array()
            .expect("runs")
            .iter()
            .map(|run| run["reason"].as_str().expect("a word"))
            .collect();
        assert!(
            reasons.contains("no-print") && reasons.contains("expired"),
            "{reasons:?}"
        );
        assert!(!reasons.contains("vendor-hole"), "{reasons:?}");
    }

    /// **A HALTED MINUTE IS NEITHER OWED NOR MISSING, AND THE MONTH NAMES THE
    /// HALT EITHER WAY.** D-3320, D-3336 §1. A series holding every minute of
    /// 2020-03-13, the halt's included, is not over-full: the halt's minutes
    /// are held inside it. One holding none of the halt's minutes loses none:
    /// they are `halt-suspected`, not `vendor-hole`.
    #[tokio::test]
    async fn a_halted_minute_is_neither_owed_nor_missing_and_the_halt_is_named() {
        let month = store::path::YearMonth::new(2020, 3).expect("a month");
        let friday = i64::from(day(2020, 3, 13).days_from_epoch());
        for (tag, keep_halt) in [("gaps-halt-held", true), ("gaps-halt-absent", false)] {
            let root = store_root(tag);
            let stamps: Vec<i64> = (555..930_i64)
                .filter(|minute| keep_halt || !(562..=620).contains(minute))
                .map(|minute| second_of(friday, minute * 60))
                .collect();
            write_rows(
                &root,
                Vendor::Dhan,
                "INDEX",
                "NIFTY",
                None,
                store::path::Timeframe::MINUTE_1,
                month,
                &stamps,
            );
            let site = std::sync::Arc::new(Site::serving(&masters(tag, None, None), &root));
            let (code, body) = gaps(
                &site,
                "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2020-03",
            )
            .await;
            assert_eq!(code, axum::http::StatusCode::OK, "{tag}: {body}");
            let audited = &body["month"][0];
            let halt = &audited["halts"][0];
            assert_eq!(halt["date"], "2020-03-13", "{tag}");
            assert_eq!(halt["suspected"], true, "{tag}");
            // COUNTED APART (D-3336 §1): 09:22-10:20 on the 13th and
            // 09:59-10:56 on the 23rd, both suspected; no sourced halt.
            assert_eq!(halt["minutes"], 59, "{tag}");
            assert_eq!(audited["halts"][1]["minutes"], 58, "{tag}");
            assert_eq!(audited["halt_suspected_minutes"], 59 + 58, "{tag}");
            assert_eq!(audited["halted_minutes"], 0, "{tag}");
            assert_eq!(
                halt["held_minutes_inside"],
                if keep_halt { 59 } else { 0 },
                "{tag}"
            );
            let runs = audited["gaps"].as_array().expect("runs");
            let on_friday: Vec<&serde_json::Value> =
                runs.iter().filter(|run| run["day"] == friday).collect();
            assert!(
                on_friday.iter().all(|run| run["reason"] != "vendor-hole"),
                "{tag}: no minute of the 13th is lost: {on_friday:?}"
            );
            assert_eq!(
                on_friday
                    .iter()
                    .any(|run| run["reason"] == "halt-suspected"),
                !keep_halt,
                "{tag}: {on_friday:?}"
            );
        }
    }

    /// **THE PEER VOTE IS ONE PARAMETER AWAY, AND AN UNKNOWN DENOMINATOR IS
    /// REFUSED.**
    #[tokio::test]
    async fn the_peer_vote_is_asked_for_by_name_and_an_unknown_denominator_is_refused() {
        let root = store_root("gaps-denominator");
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-denominator", None, None),
            &root,
        ));
        let base = "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2024-01";
        let (_, record) = gaps(&site, base).await;
        assert_eq!(record["calendar"]["source"], "record");
        let (_, peers) = gaps(&site, &format!("{base}&calendar=peers")).await;
        assert_eq!(
            peers["calendar"]["source"], "table",
            "nobody voted, so the table answers"
        );
        let (code, refused) = gaps(&site, &format!("{base}&calendar=vote")).await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|why| why.contains("calendar=record"))
        );
    }

    /// The 2018-09 NIFTY 1s month audited over a store holding only the 3rd
    /// (less a 34-second hole), with `journal` as the GDFL import journal.
    async fn audited_with_journal(name: &str, journal: Option<&str>) -> serde_json::Value {
        audited_month_with_journal(name, journal, "2018-09").await
    }

    /// [`audited_with_journal`]'s store and journal, with `asked` (a month
    /// such as `2018-09`) the month the route is asked for.
    async fn audited_month_with_journal(
        name: &str,
        journal: Option<&str>,
        asked: &str,
    ) -> serde_json::Value {
        let root = store_root(name);
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let day = i64::from(day(2018, 9, 3).days_from_epoch());
        let hole = (12 * 3_600 + 21 * 60 + 40)..(12 * 3_600 + 21 * 60 + 74);
        let stamps: Vec<i64> = (33_300..55_800)
            .filter(|second| !hole.contains(second))
            .map(|second| second_of(day, second))
            .collect();
        write_rows(
            &root,
            Vendor::Gdfl,
            "INDEX",
            "NIFTY",
            None,
            store::path::Timeframe::SECOND_1,
            month,
            &stamps,
        );
        if let Some(lines) = journal {
            let path = pull::gdfl_import::journal_path(&root);
            std::fs::create_dir_all(path.parent().expect("imports/")).expect("mkdir");
            std::fs::write(&path, lines).expect("the journal");
        }
        let site = std::sync::Arc::new(Site::serving(&masters(name, None, None), &root));
        let (code, body) = gaps(
            &site,
            &format!(
                "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month={asked}&min_run=1"
            ),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{asked}: {body}");
        body["month"][0].clone()
    }

    /// The runs of `audited` on session minutes: not `closed`, not
    /// `outside-window`.
    fn owed_runs(audited: &serde_json::Value) -> Vec<serde_json::Value> {
        audited["gaps"]
            .as_array()
            .expect("minute runs")
            .iter()
            .filter(|run| run["reason"] != "closed" && run["reason"] != "outside-window")
            .cloned()
            .collect()
    }

    /// **A DAY THE IMPORT HAS NOT REACHED IS `not-imported`, NEVER A VENDOR
    /// HOLE, AND IS NOT COUNTED LOST** (R3-05, CMP-04, SC-CLOSE-2).
    ///
    /// Before: the GDFL import journal named 2018-09-03 as the last indices
    /// day, yet every later session of the month was `vendor-hole` — 17
    /// whole days of 375 lost minutes blamed on the vendor for an import that
    /// had not run, while `/dayflags.json` said "not reached by the import
    /// yet". The 3rd's own 34-second hole is still a vendor hole.
    #[tokio::test]
    async fn a_day_the_import_has_not_reached_is_not_imported_and_not_lost() {
        // THE NEWEST LINE RE-RUNS AN OLDER DAY: the reach is the furthest day
        // the journal names, never its newest line's (the operator's journal
        // ends its indices on a re-run of 2025-10-21 with 2026-09-24 reached).
        let journal = "begin indices 2018-09-03 *\n\
            done indices 2018-09-03 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
            begin indices 2018-08-31 *\n\
            done indices 2018-08-31 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n";
        let audited = audited_with_journal("gaps-not-imported", Some(journal)).await;
        assert_eq!(audited["lost_minutes"], 0, "{audited}");
        assert_eq!(audited["expected"], 375, "the 3rd alone is owed: {audited}");
        // COUNTED APART, beside the day the import reached and its source.
        assert_eq!(audited["not_imported_minutes"], 17 * 375, "{audited}");
        assert_eq!(audited["imported_to"], "2018-09-03", "{audited}");
        assert!(
            audited["imported_to_from"]
                .as_str()
                .is_some_and(|from| from.contains("import journal") && from.contains("indices")),
            "{audited}"
        );
        // EVERY OWED MINUTE PAST THE 3RD IS `not-imported`: one whole-session
        // run on each of the 17 sessions after it, and no `vendor-hole`.
        let owed = owed_runs(&audited);
        assert_eq!(owed.len(), 17, "{audited}");
        assert!(
            owed.iter()
                .all(|run| run["reason"] == "not-imported" && run["minutes"] == 375),
            "{audited}"
        );
        let seconds = audited["second_runs"].as_array().expect("second runs");
        assert_eq!(seconds[0]["date"], "2018-09-03");
        assert_eq!(
            seconds[0]["reason"], "vendor-hole",
            "the 3rd's own hole: {audited}"
        );
        assert!(
            seconds[1..]
                .iter()
                .all(|run| run["reason"] == "not-imported"),
            "{audited}"
        );
    }

    /// **A MONTH THAT BEGINS TWO OR MORE DAYS PAST THE IMPORT'S REACH IS
    /// ANSWERED, NEVER A PANIC** (R3-05 repair 4, the second agent's
    /// blocking finding). Before: the open-day scan asked the journal for
    /// `first of the month ..= the reach`, a reversed span; `BTreeMap::range`
    /// panicked and `panic = "abort"` took the whole API down — on 15 October
    /// 2026 for `month=2026-11` and for every contract expiring in November
    /// or later. A month beginning the day after the reach (the boundary that
    /// held) gave an empty span instead.
    #[tokio::test]
    async fn a_month_begun_two_or_more_days_past_the_reach_is_answered_never_a_panic() {
        // THE REACH IS 29 AUGUST: SEPTEMBER BEGINS THREE DAYS PAST IT.
        let journal = "begin indices 2018-08-29 *\n\
            done indices 2018-08-29 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n";
        let audited = audited_with_journal("gaps-past-reach", Some(journal)).await;
        assert_eq!(audited["imported_to"], "2018-08-29", "{audited}");
        assert_eq!(audited["lost_minutes"], 0, "{audited}");
        assert_eq!(
            audited["not_yet_minutes"], 0,
            "2018 has happened: {audited}"
        );
        // EVERY SESSION AFTER THE 3RD (WHICH HOLDS ITS BARS) IS NOT IMPORTED.
        assert_eq!(audited["not_imported_minutes"], 17 * 375, "{audited}");
        let owed = owed_runs(&audited);
        assert_eq!(owed.len(), 17, "{audited}");
        assert!(
            owed.iter().all(|run| run["reason"] == "not-imported"),
            "{audited}"
        );
        // A FAR MONTH, AND THE LAST ONE THE ROUTE NAMES: answered, nothing
        // lost and nothing not imported — those sessions have not happened.
        for asked in ["2099-01", "9999-12"] {
            let far =
                audited_month_with_journal(&format!("gaps-far-{asked}"), Some(journal), asked)
                    .await;
            assert_eq!(far["lost_minutes"], 0, "{asked}: {far}");
            assert_eq!(far["not_imported_minutes"], 0, "{asked}: {far}");
            assert_eq!(far["imported_to"], "2018-08-29", "{asked}: {far}");
        }
    }

    /// **A CLOSED JOURNAL LINE FOR A DAY AFTER TODAY IS NOT THE IMPORT'S
    /// REACH** (R3-05 repair 4, the second agent's follow-up 3). Before: a
    /// `done` line for 2099-01-01 made the reach 2099, so every day of
    /// September 2018 the import never reached was a lost `vendor-hole` —
    /// 17 sessions blamed on the vendor. Now the reach is the furthest closed
    /// day at or before today, and the passed-over line is named.
    #[tokio::test]
    async fn a_closed_line_after_today_is_not_the_reach() {
        let journal = "begin indices 2018-09-03 *\n\
            done indices 2018-09-03 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
            begin indices 2099-01-01 *\n\
            done indices 2099-01-01 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n";
        let audited = audited_with_journal("gaps-future-line", Some(journal)).await;
        assert_eq!(audited["imported_to"], "2018-09-03", "{audited}");
        assert_eq!(audited["lost_minutes"], 0, "{audited}");
        assert_eq!(audited["not_imported_minutes"], 17 * 375, "{audited}");
        assert!(
            audited["imported_to_from"]
                .as_str()
                .is_some_and(|from| from.contains("2099-01-01") && from.contains("passed over")),
            "{audited}"
        );
        // ONLY A FUTURE LINE: no reach at all, said, and nothing renamed.
        let only_future = "begin indices 2099-01-01 *\n\
            done indices 2099-01-01 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n";
        let alone = audited_with_journal("gaps-future-only", Some(only_future)).await;
        assert_eq!(alone["imported_to"], serde_json::Value::Null, "{alone}");
        assert_eq!(alone["not_imported_minutes"], 0, "{alone}");
        assert!(
            alone["imported_to_from"]
                .as_str()
                .is_some_and(|from| from.contains("after today")),
            "{alone}"
        );
    }

    /// **A DAY THAT HAS NOT HAPPENED YET IS `not-yet`: NEITHER LOST NOR NOT
    /// IMPORTED** (R3-05 repair 3, the second agent's follow-up). Before:
    /// `/gaps.json` for October 2026, asked on 15 October, counted 16..31
    /// October as 7,500 not-imported minutes; for a pulled feed, whose reach
    /// says nothing, the same minutes were lost `vendor-hole`. The rule is
    /// one path for every feed: an imported reach and a pulled one.
    #[test]
    fn a_day_after_today_is_not_yet_never_lost_or_not_imported() {
        let day = |d: u8| {
            i64::from(
                pull::session::Day::new(2026, 10, d)
                    .expect("a real date")
                    .days_from_epoch(),
            )
        };
        let gap = |on: i64| pull::gaps::Gap {
            day: on,
            from: 555,
            to: 929,
            reason: pull::gaps::Reason::VendorHole,
        };
        let imported = super::super::feeds_route::ImportReach {
            to: Some(day(14)),
            from: "test".to_owned(),
            open: Vec::new(),
            today: Some(day(15)),
        };
        let pulled = super::super::feeds_route::ImportReach {
            to: None,
            from: "pulled".to_owned(),
            open: Vec::new(),
            today: Some(day(15)),
        };
        for reach in [&imported, &pulled] {
            assert_eq!(run_word(&gap(day(16)), false, None, reach), NOT_YET_WORD);
            assert_eq!(run_word(&gap(day(31)), true, None, reach), NOT_YET_WORD);
            assert!(reach.not_yet(day(16)) && !reach.not_imported(day(16)));
            assert!(!reach.not_yet(day(15)));
        }
        // TODAY ITSELF, past the reach: not imported (the day has begun).
        assert_eq!(
            run_word(&gap(day(15)), false, None, &imported),
            NOT_IMPORTED_WORD
        );
        assert_eq!(run_word(&gap(day(15)), false, None, &pulled), "vendor-hole");
        // NO CLOCK: nothing is `not-yet`; the reach decides as before.
        let unclocked = super::super::feeds_route::ImportReach {
            today: None,
            ..imported.clone()
        };
        assert_eq!(
            run_word(&gap(day(16)), false, None, &unclocked),
            NOT_IMPORTED_WORD
        );
    }

    /// **A DAY WHOSE IMPORT BEGAN AND NEVER CLOSED IS `not-imported`, NEVER
    /// LOST** — in the middle of the reach and at its end (R3-05 repair 2,
    /// the second agent's follow-up 1).
    ///
    /// Before: the reach was the furthest day ANY line named, a `begin` with
    /// no closing line included, so the trailing open 6th moved the reach to
    /// the 6th and both open days were `vendor-hole`, 750 minutes lost. Now
    /// the reach is the furthest CLOSED day (the 5th), the open 4th inside
    /// it is named, and only the 5th — closed and holding nothing — is owed
    /// and lost.
    #[tokio::test]
    async fn a_day_begun_and_never_closed_is_not_imported_and_not_lost() {
        let journal = "begin indices 2018-09-03 *\n\
            done indices 2018-09-03 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
            begin indices 2018-09-04 *\n\
            begin indices 2018-09-05 *\n\
            done indices 2018-09-05 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
            begin indices 2018-09-06 *\n";
        let audited = audited_with_journal("gaps-open-day", Some(journal)).await;
        assert_eq!(audited["imported_to"], "2018-09-05", "{audited}");
        assert_eq!(
            audited["import_open_days"],
            serde_json::json!(["2018-09-04"]),
            "{audited}"
        );
        assert_eq!(audited["lost_minutes"], 375, "the 5th alone: {audited}");
        assert_eq!(audited["expected"], 2 * 375, "the 3rd and 5th: {audited}");
        assert_eq!(audited["not_imported_minutes"], 16 * 375, "{audited}");
        let owed = owed_runs(&audited);
        assert_eq!(
            owed.iter()
                .filter(|run| run["reason"] == "not-imported")
                .count(),
            16,
            "the open 4th and the 15 sessions after the 5th: {audited}"
        );
        assert_eq!(
            owed.iter()
                .filter(|run| run["reason"] == "vendor-hole")
                .count(),
            1,
            "only the closed, empty 5th is a vendor hole: {audited}"
        );
    }

    /// **WITHOUT AN IMPORT JOURNAL FOR THE SEGMENT'S KIND, NOTHING IS
    /// RENAMED**, and the month says why (R3-05, CMP-04): no journal at all,
    /// or one that names only another kind (options through 2020-09-16),
    /// leaves the same 17 days `vendor-hole` and lost — the census's held
    /// span is never taken as the import's reach.
    #[tokio::test]
    async fn without_a_journal_for_the_kind_nothing_is_renamed_and_the_month_says_why() {
        let options_only = "begin options 2020-09-16 *\n";
        for (name, journal) in [
            ("gaps-no-journal", None),
            ("gaps-other-kind", Some(options_only)),
        ] {
            let audited = audited_with_journal(name, journal).await;
            assert!(audited["imported_to"].is_null(), "{name}: {audited}");
            assert!(
                audited["imported_to_from"]
                    .as_str()
                    .is_some_and(|why| !why.is_empty()),
                "{name}: {audited}"
            );
            assert_eq!(audited["not_imported_minutes"], 0, "{name}: {audited}");
            assert_eq!(audited["lost_minutes"], 17 * 375, "{name}: {audited}");
            let owed = owed_runs(&audited);
            assert_eq!(owed.len(), 17, "{name}: {audited}");
            assert!(
                owed.iter().all(|run| run["reason"] == "vendor-hole"),
                "{name}: {audited}"
            );
        }
    }

    /// Record one month of one series in `vendor`'s census, as a landing would.
    #[expect(clippy::too_many_arguments, reason = "one census entry's fields")]
    fn record_census(
        root: &std::path::Path,
        vendor: Vendor,
        exchange: brutex_core::instrument::Exchange,
        segment: brutex_core::instrument::Segment,
        symbol: &str,
        rung: store::path::Timeframe,
        month: store::path::YearMonth,
        rows: u64,
    ) {
        let path = pull::manifest::manifest_path(root, vendor);
        let bytes = std::fs::read(&path).unwrap_or_default();
        let mut manifest =
            pull::manifest::Manifest::open_image(vendor, &bytes).expect("a manifest");
        manifest
            .record(pull::manifest::Entry {
                key: pull::manifest::EntryKey {
                    contract: None,
                    exchange,
                    segment,
                    symbol: brutex_core::symbol::Symbol::new(symbol).expect("a symbol"),
                    timeframe: rung,
                    month,
                },
                rows,
                first_ts_micros: 1,
                last_ts_micros: 2,
            })
            .expect("one month in the census");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the manifest directory");
        std::fs::write(&path, manifest.image()).expect("publish the census");
    }

    /// **A SERIES NAMED WITHOUT ITS EXCHANGE AND SEGMENT IS AUDITED UNDER THE
    /// ONE IDENTITY THE CENSUS HOLDS IT BY, AND THE ANSWER SAYS SO; NONE, OR
    /// MORE THAN ONE, IS REFUSED BY NAME.** WP-35's verification asks
    /// `/gaps.json?feed=gdfl&symbol=NIFTY&timeframe=1s&month=2018-09`, which
    /// was a 400 for its empty segment (probeapi-2, D-1200).
    #[tokio::test]
    async fn a_series_named_without_exchange_and_segment_is_found_in_the_census_or_refused() {
        use brutex_core::instrument::{Exchange, Segment};
        let root = store_root("gaps-inferred-identity");
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let third = i64::from(day(2018, 9, 3).days_from_epoch());
        let stamps: Vec<i64> = (33_300..33_310).map(|s| second_of(third, s)).collect();
        write_rows(
            &root,
            Vendor::Gdfl,
            "INDEX",
            "NIFTY",
            None,
            store::path::Timeframe::SECOND_1,
            month,
            &stamps,
        );
        record_census(
            &root,
            Vendor::Gdfl,
            Exchange::Nse,
            Segment::Index,
            "NIFTY",
            store::path::Timeframe::SECOND_1,
            month,
            10,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-inferred-identity", None, None),
            &root,
        ));
        let (code, found) = gaps(&site, "feed=gdfl&symbol=NIFTY&timeframe=1s&month=2018-09").await;
        assert_eq!(code, axum::http::StatusCode::OK, "{found}");
        assert_eq!(found["identity"]["exchange"], "NSE");
        assert_eq!(found["identity"]["segment"], "INDEX");
        assert_eq!(found["identity"]["inferred"], true);
        let (_, named) = gaps(
            &site,
            "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09",
        )
        .await;
        assert_eq!(named["identity"]["inferred"], false);
        assert_eq!(
            found["month"], named["month"],
            "the same audit, whichever way the series was named"
        );
        assert_eq!(
            found["month"][0]["held"], 1,
            "one minute holds the ten seconds"
        );

        // A MONTH THE CENSUS DOES NOT HOLD, AND A WORD NAMED ALONE, REFUSE.
        for query in [
            "feed=gdfl&symbol=NIFTY&timeframe=1s&month=2018-10",
            "feed=gdfl&symbol=NIFTY&timeframe=1min&month=2018-09",
            "feed=gdfl&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09",
        ] {
            let (code, refused) = gaps(&site, query).await;
            assert_eq!(
                code,
                axum::http::StatusCode::BAD_REQUEST,
                "{query}: {refused}"
            );
            assert!(refused["error"].is_string(), "{query}: {refused}");
        }
        // TWO IDENTITIES ARE TWO SERIES: the request is told to name one.
        record_census(
            &root,
            Vendor::Gdfl,
            Exchange::Nse,
            Segment::Cash,
            "NIFTY",
            store::path::Timeframe::SECOND_1,
            month,
            10,
        );
        let (code, ambiguous) =
            gaps(&site, "feed=gdfl&symbol=NIFTY&timeframe=1s&month=2018-09").await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{ambiguous}");
        let why = ambiguous["error"].as_str().expect("a reason");
        assert!(
            why.contains("NSE/INDEX") && why.contains("NSE/CASH"),
            "{why}"
        );
    }

    /// **A CONTRACT OWES NOTHING AFTER ITS EXPIRY DAY, AT ONE MINUTE AS AT ONE
    /// SECOND.** A NIFTY option expiring 2018-09-27 that printed every minute
    /// of its seventeen sessions lost nothing: the 28th's minutes are
    /// `expired`, taken out of what is owed and out of what is lost. They were
    /// counted as a vendor loss beside the word `expired`.
    #[tokio::test]
    async fn a_contract_owes_nothing_after_its_expiry_day() {
        let root = store_root("gaps-expired-minute");
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let expiry = i64::from(day(2018, 9, 27).days_from_epoch());
        let first = i64::from(day(2018, 9, 1).days_from_epoch());
        let stamps: Vec<i64> = (first..=expiry)
            .filter(|d| {
                matches!(
                    pull::calendar::exchange_kind_of(*d),
                    pull::calendar::DayKind::Open(_)
                )
            })
            .flat_map(|d| (555..930_i64).map(move |minute| second_of(d, minute * 60)))
            .collect();
        assert_eq!(stamps.len(), 17 * 375, "seventeen sessions up to the 27th");
        write_rows(
            &root,
            Vendor::Gdfl,
            "FNO",
            "NIFTY",
            Some("2018-09-27-1100000-CE"),
            store::path::Timeframe::MINUTE_1,
            month,
            &stamps,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-expired-minute", None, None),
            &root,
        ));
        let (code, body) = gaps(
            &site,
            "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2018-09-27-1100000-CE&timeframe=1min&month=2018-09",
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        let audited = &body["month"][0];
        assert_eq!(audited["lost_minutes"], 0, "{audited}");
        assert_eq!(audited["expected"], 17 * 375, "{audited}");
        assert_eq!(audited["held"], 17 * 375);
        let runs = audited["gaps"].as_array().expect("runs");
        let friday = i64::from(day(2018, 9, 28).days_from_epoch());
        assert!(
            runs.iter()
                .filter(|run| run["day"] == friday)
                .all(|run| run["reason"] == "expired"),
            "{runs:?}"
        );
        assert!(runs.iter().any(|run| run["day"] == friday), "{runs:?}");
    }

    /// **A PEER VOTE STILL BUILDING SAYS WHAT IT IS DOING, NEVER "0 OF 0".**
    /// DB-N19: a vote walks every feed's census entries before it knows how
    /// many peer series it owes, and an FNO audit owes none at all.
    #[test]
    fn a_building_vote_says_what_it_is_doing_and_audits_nothing() {
        let asked = Addressed::parse(
            "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-09-26-1300000-CE&timeframe=1s&month=2019-09",
        )
        .unwrap();
        let body = |phase: &str, counted: u64, owed: u64| {
            let (status, _, body) = vote_building(
                &asked,
                phase,
                "request",
                std::time::Duration::from_millis(40),
                Some(std::time::Duration::from_millis(30)),
                counted,
                owed,
            );
            assert_eq!(status, axum::http::StatusCode::ACCEPTED);
            serde_json::from_str::<serde_json::Value>(&body).unwrap()
        };
        let walking = body("deriving", 0, 0);
        assert_eq!(walking["status"], "building");
        assert_eq!(walking["calendar"], "peers");
        assert_eq!(walking["symbol"], "NIFTY");
        assert_eq!(walking["since_ms"], 40);
        assert_eq!(walking["reading_ms"], 30);
        let reason = walking["reason"].as_str().unwrap();
        assert!(
            reason.contains("walking every feed's census for the peer series"),
            "{reason}"
        );
        assert!(!reason.contains("0 of the 0"), "{reason}");
        assert!(
            walking.get("lost_minutes").is_none(),
            "no minute is audited"
        );
        let deriving = body("deriving", 57, 343);
        let reason = deriving["reason"].as_str().unwrap();
        assert!(
            reason.contains("it has derived 57 of the 343 peer series"),
            "{reason}"
        );
        assert_eq!(deriving["series_owed"], 343);
        let queued = body("queued", 0, 0);
        assert!(
            queued["reason"].as_str().unwrap().contains("it is queued"),
            "{queued}"
        );
    }

    // ---- R3-06: the month summary (D-4980, DB-A30, DB-G47) ----

    /// The stamps of every minute `owed` lists on `day`.
    fn minutes_of(day: i64, owed: &[u16]) -> Vec<i64> {
        owed.iter()
            .map(|minute| second_of(day, i64::from(*minute) * 60))
            .collect()
    }

    /// The bar file of one series' month under `root`.
    fn bars_path(
        root: &std::path::Path,
        vendor: Vendor,
        segment: &str,
        symbol: &str,
        rung: store::path::Timeframe,
        month: store::path::YearMonth,
    ) -> std::path::PathBuf {
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor,
            exchange: "NSE",
            segment,
            symbol,
            contract: None,
            timeframe: rung,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        path.to_path_buf(root)
    }

    /// `body` without the members that say HOW it was had, so a kept and a
    /// built answer can be compared for WHAT they say.
    fn what_it_says(mut body: serde_json::Value) -> serde_json::Value {
        if let Some(object) = body.as_object_mut() {
            object.remove("summaries");
        }
        if let Some(months) = body["month"].as_array_mut() {
            for month in months {
                if let Some(object) = month.as_object_mut() {
                    // HOW, NOT WHAT: a kept month read no record and counts
                    // none (`null`); a built one counts what it read. Each is
                    // checked against its source, then set aside.
                    let unread = object.remove("unreadable_records");
                    let kept =
                        object.get("summary").and_then(serde_json::Value::as_str) == Some("kept");
                    assert_eq!(
                        unread.as_ref().is_some_and(serde_json::Value::is_null),
                        kept,
                        "unreadable_records is null exactly when the summary was kept: {unread:?}"
                    );
                    object.remove("summary");
                    object.remove("summary_fallback");
                    object.remove("bar_records_read");
                }
            }
        }
        body
    }

    /// **THE GAP AUDIT READS NO BAR WHEN SUMMARIES EXIST** (R3-06, DB-A30).
    ///
    /// Counted twice, two ways. The answer counts the bar records it read,
    /// and says zero. And the proof that does not trust that count: every
    /// record in the middle of both months is then overwritten with garbage
    /// in place (the test's own store), and the answer does not move by one
    /// byte and still counts no record it did not read (`null`) -- a reader
    /// that read a bar would have read garbage. Only when the summary itself is spoiled are the
    /// bars read, named as the fallback, and the garbage shows.
    #[tokio::test]
    #[expect(
        clippy::too_many_lines,
        reason = "one fixture, the kept answer, the garbage proof and the spoiled \
                  fallback, in the order the doc states them"
    )]
    async fn the_gap_audit_reads_no_bar_when_summaries_exist() {
        let root = store_root("gaps-summary-no-bar");
        let month = store::path::YearMonth::new(2018, 9).expect("a month");
        let monday = i64::from(day(2018, 9, 3).days_from_epoch());
        let hole = (12 * 3_600 + 21 * 60 + 40)..(12 * 3_600 + 21 * 60 + 74);
        let seconds: Vec<i64> = (monday..monday + 2)
            .flat_map(|day| (33_300..55_800).map(move |second| (day, second)))
            .filter(|(_, second)| !hole.contains(second))
            .map(|(day, second)| second_of(day, second))
            .collect();
        let minutes: Vec<i64> = (monday..monday + 5)
            .flat_map(|day| {
                let owed = crate::calendar_of::record::owed_minutes(day).unwrap_or_default();
                minutes_of(day, &owed)
            })
            .filter(|stamp| (stamp / 60_000_000) % 97 != 0)
            .collect();
        let one_second = store::path::Timeframe::SECOND_1;
        let one_minute = store::path::Timeframe::MINUTE_1;
        write_rows(
            &root,
            Vendor::Gdfl,
            "INDEX",
            "NIFTY",
            None,
            one_second,
            month,
            &seconds,
        );
        write_rows(
            &root,
            Vendor::Zerodha,
            "INDEX",
            "NIFTY",
            None,
            one_minute,
            month,
            &minutes,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-summary-no-bar", None, None),
            &root,
        ));
        let asks = [
            (
                "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2018-09",
                bars_path(&root, Vendor::Gdfl, "INDEX", "NIFTY", one_second, month),
                seconds.len(),
            ),
            (
                "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2018-09",
                bars_path(&root, Vendor::Zerodha, "INDEX", "NIFTY", one_minute, month),
                minutes.len(),
            ),
        ];
        for (query, bars, rows) in asks {
            let (code, kept) = gaps(&site, query).await;
            assert_eq!(code, axum::http::StatusCode::OK, "{kept}");
            assert_eq!(kept["month"][0]["summary"], "kept", "{query}: {kept}");
            assert_eq!(kept["month"][0]["bar_records_read"], 0, "{query}");
            assert!(kept["month"][0]["summary_fallback"].is_null(), "{query}");
            assert_eq!(kept["summaries"]["kept"], 1, "{query}");
            assert_eq!(kept["summaries"]["bar_records_read"], 0, "{query}");
            // NOT READ, SO NOT CLAIMED: a kept answer read no record and
            // says so with `null`, never a `0` it did not count.
            assert!(
                kept["month"][0]["unreadable_records"].is_null(),
                "{query}: {kept}"
            );
            assert!(kept["lost_minutes"].as_u64().is_some(), "{query}: {kept}");

            // GARBAGE IN EVERY MIDDLE RECORD: the answer cannot move.
            let layout = store::layout::Layout::CURRENT;
            let last = u64::try_from(rows).expect("rows") - 600;
            let from = layout.offset_of(100).expect("an offset");
            let to = layout.offset_of(last).expect("an offset");
            let mut image = std::fs::read(&bars).expect("the bar file");
            for byte in image
                .get_mut(
                    usize::try_from(from).expect("offset")..usize::try_from(to).expect("offset"),
                )
                .expect("the records are in the file")
            {
                *byte = 0xA5;
            }
            std::fs::write(&bars, &image).expect("spoil the test's own records");
            let (_, spoiled_bars) = gaps(&site, query).await;
            assert_eq!(
                spoiled_bars, kept,
                "{query}: a kept summary answers without reading one bar"
            );

            // A SPOILED SUMMARY IS NAMED, THE BARS ARE READ, AND THE DAMAGE SHOWS.
            // The magic and five bytes: shorter than any section, so `torn`.
            let beside = store::month_summary::path_beside(&bars);
            std::fs::write(&beside, b"BRUTEXU1 torn").expect("spoil the test's own summary");
            let (_, built) = gaps(&site, query).await;
            let audited = &built["month"][0];
            assert_eq!(audited["summary"], "built", "{query}: {built}");
            assert_eq!(
                audited["bar_records_read"],
                u64::try_from(rows).expect("rows")
            );
            assert_eq!(built["summaries"]["built"], 1, "{query}");
            assert!(
                audited["summary_fallback"]
                    .as_str()
                    .is_some_and(|why| why.starts_with("torn")),
                "{query}: {audited}"
            );
            assert!(
                audited["unreadable_records"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
                "{query}: the garbage the summary never read is read now: {audited}"
            );
        }
    }

    /// **A KEPT SUMMARY'S ANSWER IS THE FULL READ'S ANSWER, IN EVERY EXTREME**
    /// (R3-06). Each month below is asked twice: once off the summary its
    /// writer kept, once with that summary spoiled so the bars are read whole
    /// and summarised by the same library call. What the two say must be
    /// identical -- only how it was had may differ.
    #[tokio::test]
    async fn a_kept_summary_answers_what_a_full_read_answers_in_every_extreme() {
        let root = store_root("gaps-summary-equal");
        let september = store::path::YearMonth::new(2018, 9).expect("a month");
        let march = store::path::YearMonth::new(2020, 3).expect("a month");
        let monday = i64::from(day(2018, 9, 3).days_from_epoch());
        let one_second = store::path::Timeframe::SECOND_1;
        let one_minute = store::path::Timeframe::MINUTE_1;
        let owed = |day: i64| crate::calendar_of::record::owed_minutes(day).unwrap_or_default();
        // Holes of one minute, of a run, and of whole days.
        let holed: Vec<i64> = (monday..monday + 12)
            .filter(|day| day % 5 != 0)
            .flat_map(|day| minutes_of(day, &owed(day)))
            .filter(|stamp| (stamp / 60_000_000) % 41 != 0)
            .collect();
        // The halt month, every owed minute held but the halt's.
        let first_march = i64::from(day(2020, 3, 1).days_from_epoch());
        let halted: Vec<i64> = (first_march..first_march + 31)
            .flat_map(|day| minutes_of(day, &owed(day)))
            .collect();
        // A one-second index month: one dense day, one sparse afternoon, a
        // print in a minute the calendar does not owe.
        let dense: Vec<i64> = (33_300..55_800)
            .map(|second| second_of(monday, second))
            .chain(
                (40_000..41_000)
                    .step_by(3)
                    .map(|s| second_of(monday + 1, s)),
            )
            .chain(std::iter::once(second_of(monday + 1, 60_000)))
            .collect();
        // A sparse one-second index month: its summary keeps no seconds.
        let sparse: Vec<i64> = (0..1_100)
            .map(|k| second_of(monday + 1 + k / 300, 33_300 + (k % 300) * 70))
            .collect();
        let series: [(
            Vendor,
            &str,
            store::path::Timeframe,
            store::path::YearMonth,
            &[i64],
        ); 5] = [
            (Vendor::Zerodha, "NIFTY", one_minute, september, &holed),
            (Vendor::Zerodha, "BANKNIFTY", one_minute, march, &halted),
            (Vendor::Gdfl, "NIFTY", one_second, september, &dense),
            (Vendor::Gdfl, "BANKNIFTY", one_second, september, &sparse),
            (Vendor::Gdfl, "FINNIFTY", one_second, march, &halted),
        ];
        for (vendor, symbol, rung, month, stamps) in series {
            write_rows(&root, vendor, "INDEX", symbol, None, rung, month, stamps);
        }
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-summary-equal", None, None),
            &root,
        ));
        for (vendor, symbol, rung, month, stamps) in series {
            let query = format!(
                "feed={}&exchange=NSE&segment=INDEX&symbol={symbol}&timeframe={}&month={month}",
                vendor.as_str(),
                rung.as_str()
            );
            let (code, kept) = gaps(&site, &query).await;
            assert_eq!(code, axum::http::StatusCode::OK, "{query}: {kept}");
            let bars = bars_path(&root, vendor, "INDEX", symbol, rung, month);
            let source = kept["month"][0]["summary"].clone();
            if rung == one_second && symbol != "NIFTY" {
                // AN INDEX AUDIT NEEDS SECONDS A SPARSE SUMMARY DOES NOT KEEP:
                // named, and read from the bars. Both the sparse afternoon
                // (BANKNIFTY) and the halt month held at one print a minute
                // (FINNIFTY) hold under half the seconds they span (D-4980).
                assert_eq!(source, "built", "{query}: {kept}");
                assert!(
                    kept["month"][0]["summary_fallback"]
                        .as_str()
                        .is_some_and(|why| why.contains("no seconds index")),
                    "{kept}"
                );
            } else {
                assert_eq!(source, "kept", "{query}: {kept}");
            }
            if rung == one_minute {
                // A MINUTE MONTH HOLDS ITS ROWS.
                assert_eq!(kept["month"][0]["held"], stamps.len() as u64, "{query}");
            }
            std::fs::write(store::month_summary::path_beside(&bars), b"spoiled")
                .expect("spoil the test's own summary");
            let (_, built) = gaps(&site, &query).await;
            assert_eq!(built["month"][0]["summary"], "built", "{query}");
            assert_eq!(
                what_it_says(kept),
                what_it_says(built),
                "{query}: the kept and the read answer say the same"
            );
        }
    }

    /// **A RECORDED HALT IS NOT OWED, ON EVERY ROUTE** (R3-06, DB-G47).
    ///
    /// `/calendar.json` owes a halt day what `calendar_record::owed_minutes`
    /// lists and `/dayflags.json` what `pull::calendar::phases_of` calls
    /// continuous; `/gaps.json` must owe exactly that on every door it audits
    /// with -- the spot index's, the derivatives venue's, the folded one-second
    /// index's -- and lose nothing on a series that holds every owed minute.
    #[tokio::test]
    #[expect(
        clippy::too_many_lines,
        reason = "one halt day asked on every door the audit owes with, each \
                  beside the calendar route it must agree with"
    )]
    async fn a_recorded_halt_is_not_owed_on_every_route() {
        let root = store_root("gaps-halt-every-route");
        let march = store::path::YearMonth::new(2020, 3).expect("a month");
        let first = i64::from(day(2020, 3, 1).days_from_epoch());
        let halts: Vec<i64> = pull::calendar::HALTS
            .iter()
            .map(|halt| halt.day)
            .filter(|halt| (first..first + 31).contains(halt))
            .collect();
        assert_eq!(halts.len(), 2, "2020-03-13 and 2020-03-23");
        for halt in &halts {
            let record = crate::calendar_of::record::owed_minutes(*halt).expect("a sized day");
            for venue in [
                pull::vendor::Venue::NseIndex,
                pull::vendor::Venue::NseCash,
                pull::vendor::Venue::NseDerivatives,
            ] {
                let phases = pull::calendar::phases_of(
                    venue,
                    *halt,
                    pull::calendar::CasEligibility::Unknown,
                );
                assert_eq!(
                    usize::from(phases.minutes(pull::calendar::Phase::Continuous)),
                    record.len(),
                    "{venue:?} {}: /dayflags.json and /calendar.json owe one count",
                    iso(*halt)
                );
            }
        }
        let owed = |day: i64| crate::calendar_of::record::owed_minutes(day).unwrap_or_default();
        let held: Vec<i64> = (first..first + 26)
            .flat_map(|day| minutes_of(day, &owed(day)))
            .collect();
        let seconds: Vec<i64> = held
            .iter()
            .filter(|stamp| halts.contains(&ist_second(**stamp).0))
            .flat_map(|minute| (0..60).map(move |s| minute + s * 1_000_000))
            .collect();
        let owed_total: u64 = (first..first + 31).map(|day| owed(day).len() as u64).sum();
        let owed_halts: u64 = halts.iter().map(|day| owed(*day).len() as u64).sum();
        write_rows(
            &root,
            Vendor::Zerodha,
            "INDEX",
            "NIFTY",
            None,
            store::path::Timeframe::MINUTE_1,
            march,
            &held,
        );
        write_rows(
            &root,
            Vendor::Zerodha,
            "FNO",
            "NIFTY",
            Some("2020-03-26-1000000-CE"),
            store::path::Timeframe::MINUTE_1,
            march,
            &held,
        );
        write_rows(
            &root,
            Vendor::Gdfl,
            "INDEX",
            "NIFTY",
            None,
            store::path::Timeframe::SECOND_1,
            march,
            &seconds,
        );
        let site = std::sync::Arc::new(Site::serving(
            &masters("gaps-halt-every-route", None, None),
            &root,
        ));
        for query in [
            "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2020-03",
            "feed=zerodha&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2020-03-26-1000000-CE&timeframe=1min&month=2020-03",
            "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2020-03",
        ] {
            let (code, body) = gaps(&site, query).await;
            assert_eq!(code, axum::http::StatusCode::OK, "{query}: {body}");
            let audited = &body["month"][0];
            let runs = audited["gaps"].as_array().expect("runs");
            for halt in &halts {
                let lost_on_halt: u64 = runs
                    .iter()
                    .filter(|run| run["day"] == *halt && run["reason"] == "vendor-hole")
                    .map(|run| run["minutes"].as_u64().unwrap_or(0))
                    .sum();
                assert_eq!(lost_on_halt, 0, "{query} {}: {runs:?}", iso(*halt));
            }
            let halts_listed = audited["halts"].as_array().expect("halts");
            assert_eq!(halts_listed.len(), 2, "{query}: {audited}");
            if query.contains("1min") && !query.contains("contract") {
                // EVERY OWED MINUTE OF THE MONTH: the halt days' owed exactly
                // the record's, the days after the 26th lost whole.
                assert_eq!(audited["expected"], owed_total, "{query}");
                assert_eq!(
                    audited["lost_minutes"],
                    owed_total - held.len() as u64,
                    "{query}"
                );
            } else if query.contains("contract") {
                // NOTHING IS OWED AFTER THE 26th EXPIRY, AND NOTHING LOST.
                assert_eq!(audited["lost_minutes"], 0, "{query}: {audited}");
                assert_eq!(audited["expected"], held.len() as u64, "{query}");
            } else {
                // ONE-SECOND, FOLDED: the halt days alone are held.
                assert_eq!(
                    audited["lost_minutes"],
                    owed_total - owed_halts,
                    "{query}: {audited}"
                );
            }
        }
    }
}

/// Tests written to kill mutants a cloud mutation pass left alive in this
/// file (`reports/cloud-mut-api-a-20261016.md`). Each names the mutant.
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod cloud_mutant_tests {
    use super::*;
    use brutex_core::vendor::Vendor;
    use store::path::YearMonth;

    fn month(year: u16, month: u8) -> YearMonth {
        YearMonth::new(year, month).expect("a real month")
    }

    fn epoch_day(year: u16, month: u8, day: u8) -> i64 {
        i64::from(
            Day::new(year, month, day)
                .expect("a real day")
                .days_from_epoch(),
        )
    }

    /// The one-second month summary of `rows`, all inside the month of the
    /// IST epoch day `day`: what the audit reads in place of the bars since
    /// R3-06, folded exactly as the writer folds them.
    fn summary_of(day: i64, rows: &[store::format::Bar]) -> store::month_summary::MonthSummary {
        let civil = Day::from_days(u32::try_from(day).expect("a day after the epoch"))
            .expect("a day the calendar names");
        let mut summary =
            store::month_summary::MonthSummary::new(0, 1, month(civil.year(), civil.month()));
        for bar in rows {
            summary.fold(bar);
        }
        summary
    }

    /// An import reach that says nothing: no journal, no clock, so no minute
    /// is `not-imported` or `not-yet`, as for a pulled feed read with no
    /// clock.
    fn silent_reach() -> super::super::feeds_route::ImportReach {
        super::super::feeds_route::ImportReach {
            to: None,
            from: "test".to_owned(),
            open: Vec::new(),
            today: None,
        }
    }

    /// The `calendar` member of one `/gaps.json` body over `span`, audited
    /// against a denominator from `source` vouching for `bounds`.
    fn calendar_of(
        source: &'static str,
        bounds: (i64, i64),
        span: &[YearMonth],
    ) -> serde_json::Value {
        let denominator = Denominator {
            calendar: None,
            source,
            bounds,
            from: Vec::new(),
            unreadable: Vec::new(),
        };
        let identity = Identity {
            json: r#""identity":{}"#.to_owned(),
            one_second: false,
        };
        let body = gaps_body(&[], span, &denominator, false, &identity);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("a JSON body");
        parsed["calendar"].clone()
    }

    /// **A PEER VOTE NOBODY CAST STILL NAMES WHO VOTED AND WHO COULD NOT BE
    /// READ.** With no calendar agreed the denominator is the typed table, and
    /// the readings and the unreadable peers ride on it to the answer. Kills
    /// `delete field from` and `delete field unreadable` in
    /// `Denominator::peers`: either would answer `voted_by: []` or
    /// `unreadable: []`, the same answer as a store with no peers.
    #[test]
    fn a_vote_with_no_calendar_keeps_its_readings_and_its_unreadable_peers() {
        let denominator = Denominator::peers(PeerCalendar {
            calendar: None,
            from: vec!["dhan:NIFTY".to_owned()],
            unreadable: vec!["groww".to_owned()],
        });
        assert_eq!(denominator.source, "table");
        assert!(denominator.calendar.is_none());
        assert_eq!(denominator.from, vec!["dhan:NIFTY".to_owned()]);
        assert_eq!(denominator.unreadable, vec!["groww".to_owned()]);
    }

    /// **A HALT THAT RUNS TO THE CLOSE LEAVES ONE WINDOW, AND ONE WINDOW IS A
    /// SESSION.** 2021-02-24's outage halts minutes 700..=944, past the 15:29
    /// close, so carving it out of a regular session leaves 09:15..=11:39 —
    /// one run. `> with <` in `carved` refused every carving with fewer runs
    /// than a session can hold, so that day's owed minutes read unmeasured.
    #[test]
    fn a_halt_that_runs_to_the_close_carves_a_one_window_session() {
        let day = pull::calendar::SYSTEMS_OUTAGE_DAY;
        let carved = carved(day, pull::calendar::Session::full()).expect("one window is a session");
        assert_eq!(carved.count, 1);
        assert!(carved.expects(555), "09:15 is owed");
        assert!(carved.expects(699), "11:39 is owed");
        assert!(!carved.expects(700), "the halt is carved out");
        // A day with no recorded halt is handed back untouched.
        let plain = epoch_day(2025, 5, 2);
        assert_eq!(carved_or_none(plain), Some(pull::calendar::Session::full()));
    }

    fn carved_or_none(day: i64) -> Option<pull::calendar::Session> {
        carved(day, pull::calendar::Session::full())
    }

    /// **`stale` IS TODAY PAST THE SOURCE'S LAST DAY, AND NEVER FOR A PEER
    /// VOTE.** A record whose last day is yesterday is stale; one ending today
    /// or later is not; a peer vote is never stale, whatever its bounds. Kills
    /// `!= with ==` and `&& with ||` at 670-671 and the three mutants of the
    /// `>` that compares today with the last day.
    #[test]
    fn stale_is_today_past_the_last_day_and_never_a_peer_vote() {
        let today = i64::from(crate::ingest::today_ist().expect("today").days_from_epoch());
        let span = [month(2025, 5)];
        let first = epoch_day(2025, 5, 1);
        for (source, last, stale) in [
            ("record", today - 1, true),
            ("record", today, false),
            ("record", today + 1, false),
            ("table", today - 1, true),
            ("peers", today - 1, false),
            ("peers", today + 1, false),
        ] {
            let calendar = calendar_of(source, (first, last), &span);
            assert_eq!(
                calendar["stale"], stale,
                "{source} ending {last} (today {today})"
            );
        }
    }

    /// **`covers_span` IS THE WHOLE SPAN INSIDE WHAT THE SOURCE VOUCHES FOR**:
    /// its first month's first day on or after the source's first day, AND its
    /// last month's last day on or before the source's last. Kills `>= with <`
    /// at 679 and `&& with ||` at 680.
    #[test]
    fn a_span_is_covered_only_when_both_its_ends_are_inside_the_source() {
        let (may_first, may_last) = (epoch_day(2025, 5, 1), epoch_day(2025, 5, 31));
        let span = [month(2025, 5)];
        for (bounds, covered) in [
            ((may_first, may_last), true),
            ((may_first - 10, may_last + 10), true),
            ((may_first + 1, may_last), false),
            ((may_first, may_last - 1), false),
            ((may_first + 1, may_last - 1), false),
        ] {
            assert_eq!(
                calendar_of("record", bounds, &span)["covers_span"],
                covered,
                "{bounds:?}"
            );
        }
    }

    /// **A VOTE THAT WAITS ON THE START-UP CENSUS SAYS SO**, not that it is
    /// walking the census. Kills `delete match arm "census"` in
    /// `vote_building`: the answer then fell through to the "walking every
    /// feed's census" sentence of a build with nothing owed yet.
    #[test]
    fn a_vote_held_by_the_start_up_census_names_that_wait() {
        let asked = Addressed {
            vendor: Vendor::Dhan,
            month: month(2025, 5),
            timeframe: store::path::Timeframe::MINUTE_1,
            contract: None,
            exchange: "NSE".to_owned(),
            segment: "INDEX".to_owned(),
            symbol: "NIFTY".to_owned(),
        };
        let (status, _, body) = vote_building(
            &asked,
            "census",
            "start-up",
            std::time::Duration::from_millis(5),
            None,
            0,
            0,
        );
        assert_eq!(status, axum::http::StatusCode::ACCEPTED);
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let reason = body["reason"].as_str().expect("a reason");
        assert!(reason.contains("reading its census at start"), "{reason}");
        assert!(!reason.contains("walking"), "{reason}");
    }

    /// **2021-02-24 OWES NO SECOND** (D-0420): the index's publication window
    /// that day is unproved, so a one-second audit counts its rows as outside
    /// and audits no second of it. Kills the match guard
    /// `day != SYSTEMS_OUTAGE_DAY -> true` in `second_audit`, which audited
    /// the day against its carved session.
    #[test]
    fn the_outage_day_owes_no_second() {
        let day = pull::calendar::SYSTEMS_OUTAGE_DAY;
        let at = |second: i64| store::format::Bar {
            ts_micros: (day * 86_400 - 19_800 + second) * 1_000_000,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: store::format::OI_NULL,
        };
        // 09:15:00 and 09:15:01 IST, against the calendar of record, which
        // opens the day with its halt carved out (one window, 09:15..=11:39).
        let rows = [at(33_300), at(33_301)];
        let record = record_calendar(day, day);
        assert!(
            matches!(
                record.kind_of(day),
                pull::calendar::DayKind::Open(session) if session.expects(555)
            ),
            "the record opens the day, so only the outage rule can owe it nothing"
        );
        let audit = second_audit(&summary_of(day, &rows), day, day, Some(&record), 1);
        assert_eq!(audit.expected, 0, "no second of the day is owed");
        assert_eq!(audit.held, 0);
        assert_eq!(
            audit.outside, 2,
            "both rows are held outside any owed second"
        );
        assert!(audit.days.is_empty(), "the day is not audited at all");
    }

    /// **A DAY'S LONGEST RUN IS THE FIRST OF EQUAL LENGTH.** Two missing runs
    /// of three seconds each: the day names the earlier. Kills `> with >=` in
    /// `close_run`, which let the later of two equal runs replace it.
    #[test]
    fn of_two_equally_long_runs_the_earlier_is_the_longest() {
        let mut audit = SecondAudit::default();
        let mut day = SecondDay {
            day: 1,
            expected: 0,
            held: 0,
            missing: 0,
            longest: None,
        };
        for run in [(100, 102), (200, 202)] {
            let mut open = Some(run);
            close_run(&mut audit, &mut day, &mut open, 1);
            assert_eq!(open, None, "a closed run is taken");
        }
        assert_eq!(day.longest, Some((100, 3)));
        assert_eq!(audit.run_count, 2);
        // A strictly longer run still replaces it.
        let mut open = Some((300, 310));
        close_run(&mut audit, &mut day, &mut open, 1);
        assert_eq!(day.longest, Some((300, 11)));
    }

    fn subject(
        segment: &str,
        contract: Option<&str>,
        timeframe: store::path::Timeframe,
    ) -> Addressed {
        Addressed {
            vendor: Vendor::Gdfl,
            month: month(2026, 8),
            timeframe,
            contract: contract
                .map(|text| brutex_core::instrument::Contract::parse(text).expect("a contract")),
            exchange: "NSE".to_owned(),
            segment: segment.to_owned(),
            symbol: "NIFTY".to_owned(),
        }
    }

    /// **A VOTE THAT COULD NOT READ EVERY CENSUS IS ANSWERED AND NEVER KEPT**
    /// (DB-N19): it names the unreadable feed, and the next audit reads again.
    /// With every census readable the same vote may be kept. Kills `&& with
    /// ||` in `peer_vote_reading`'s `keep`, which kept a vote that named an
    /// unreadable feed for as long as the stamps did not move.
    #[test]
    fn a_vote_that_could_not_read_every_census_is_never_kept() {
        let root = crate::server::tests::store_root("cloud-gaps-unkept");
        let masters = crate::server::tests::masters("cloud-gaps-unkept", None, None);
        let manifest = pull::manifest::manifest_path(&root, Vendor::Groww);
        std::fs::create_dir_all(manifest.parent().expect("a manifest directory"))
            .expect("the manifest directory");
        std::fs::write(&manifest, b"these bytes are no census").expect("a refused manifest");
        let asked = subject("INDEX", None, store::path::Timeframe::MINUTE_1);
        let site = Site::load(&masters, &root);
        let (keep, vote) = peer_vote_reading(&site, &asked, census_now_stamped);
        assert_eq!(vote.unreadable, vec!["groww".to_owned()], "named");
        assert!(
            keep.is_none(),
            "a vote naming an unreadable feed is not kept"
        );

        std::fs::remove_file(&manifest).expect("the refused manifest goes");
        let site = Site::load(&masters, &root);
        let (keep, vote) = peer_vote_reading(&site, &asked, census_now_stamped);
        assert!(vote.unreadable.is_empty(), "{:?}", vote.unreadable);
        assert!(keep.is_some(), "a whole read may be kept");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A ONE-SECOND OPTION IS AUDITED ON THE DERIVATIVES VENUE'S HOURS**
    /// (hunt-pull-1, D-1529): from 2026-08-03 its session runs to 15:39, so a
    /// day it printed only at the open owes nothing it lacks and lists a
    /// `no-print` run to minute 939. `== with !=` in `audit_one` sent every
    /// trade-driven series that is NOT cash to the cash calendar instead, and
    /// its runs stopped at 15:29 (minute 929): no test audited an option on
    /// a day those hours differ.
    #[tokio::test]
    async fn a_one_second_option_is_audited_on_the_derivatives_hours() {
        let root = crate::server::tests::store_root("cloud-gaps-option-hours");
        let contract = "2026-08-27-2500000-CE";
        let tuesday = epoch_day(2026, 8, 4);
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "FNO",
            symbol: "NIFTY",
            contract: brutex_core::instrument::Contract::parse(contract),
            timeframe: store::path::Timeframe::SECOND_1,
            month: month(2026, 8),
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        let id = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
            .expect("low 32 bits");
        let mut file = store::file::BarFile::open_or_create(&root, path, id).expect("a file");
        let print = store::format::Bar {
            ts_micros: (tuesday * 86_400 - 19_800 + 33_300) * 1_000_000,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        };
        file.append(&[print]).expect("one print at 09:15:00");
        drop(file);
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters("cloud-gaps-option-hours", None, None),
            &root,
        ));
        let uri: axum::http::Uri = format!(
            "/gaps.json?feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract={contract}\
             &timeframe=1s&month=2026-08"
        )
        .parse()
        .expect("a URI");
        let (code, _, body) = gaps_json(axum::extract::State(site), uri).await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let audited = &body["month"][0];
        assert_eq!(audited["trade_driven"], true, "{audited}");
        let runs = audited["gaps"].as_array().expect("runs");
        let tuesday_runs: Vec<(i64, &str)> = runs
            .iter()
            .filter(|run| run["day"] == tuesday)
            .map(|run| {
                (
                    run["to"].as_i64().expect("a minute"),
                    run["reason"].as_str().expect("a word"),
                )
            })
            .collect();
        assert!(
            tuesday_runs.contains(&(939, "no-print")),
            "the derivatives session runs to 15:39: {tuesday_runs:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- lines no test ran (coverage) ------------------------------------

    /// **A RANGE PAST ITS CEILING STOPS AND SAYS SO, AND DECEMBER ROLLS INTO
    /// THE NEXT YEAR.** Runs `span_of`'s truncation return and `next_month`'s
    /// December arm.
    #[test]
    fn a_span_rolls_december_and_stops_at_its_ceiling_by_name() {
        let (span, truncated) = span_of(month(2024, 12), month(2025, 2), 240);
        assert_eq!(span, vec![month(2024, 12), month(2025, 1), month(2025, 2)]);
        assert!(!truncated);
        let (span, truncated) = span_of(month(2000, 1), month(2030, 12), 12);
        assert_eq!(span.len(), 12);
        assert_eq!(span.last(), Some(&month(2000, 12)));
        assert!(truncated, "the ceiling is said, never a silent take");
    }

    /// **THE RECORD'S CALENDAR KEEPS A SESSION OF UNKNOWN LENGTH UNKNOWN, AND
    /// A DAY PAST WHAT IT SOURCES UNMEASURED.** A session the record says
    /// traded with a length it does not know owes nothing and loses nothing;
    /// a day after `SOURCED_LAST_DAY` is not called closed. Runs
    /// `record_calendar`'s `OpenLengthUnmeasured` and `Unmeasured` arms.
    #[test]
    fn the_record_keeps_unknown_lengths_and_unsourced_days_unmeasured() {
        use pull::calendar::DayKind;
        // THE FIRST SUCH DAY THE RECORD HOLDS, found rather than named: a
        // special session sourced later moves it, and the arm stays covered.
        let muhurat = (pull::calendar::SOURCED_FIRST_DAY..=pull::calendar::SOURCED_LAST_DAY)
            .find(|day| pull::calendar::exchange_kind_of(*day) == DayKind::OpenLengthUnmeasured)
            .expect("the record holds a session of unmeasured length");
        let record = record_calendar(muhurat - 3, muhurat + 3);
        assert_eq!(record.kind_of(muhurat), DayKind::OpenLengthUnmeasured);

        let past = pull::calendar::SOURCED_LAST_DAY + 3;
        assert_eq!(pull::calendar::exchange_kind_of(past), DayKind::Unmeasured);
        let record = record_calendar(pull::calendar::SOURCED_LAST_DAY - 5, past);
        assert_eq!(record.kind_of(past), DayKind::Unmeasured);
    }

    /// **A CARVING THAT WOULD NEED MORE WINDOWS THAN A SESSION HOLDS IS A
    /// SESSION OF UNKNOWN LENGTH**: 2020-03-13's halt cutting the first of two
    /// windows leaves three runs. Runs `carved`'s `None`.
    #[test]
    fn a_carving_past_the_window_limit_is_a_session_of_unknown_length() {
        let two_windows = pull::calendar::Observed::from_runs(18_334, &[(555, 700), (800, 929)])
            .session
            .expect("two windows are a session");
        assert_eq!(carved(18_334, two_windows), None);
    }

    /// **A SOURCED HALT'S MINUTES ARE `halted`, A SUSPECTED ONE'S
    /// `halt-suspected`, AND A CONTRACT'S MINUTES PAST EXPIRY `expired`.**
    /// Runs `run_word`'s sourced-halt arm, which no fixture reached.
    #[test]
    fn a_minute_inside_a_sourced_halt_is_halted() {
        use pull::gaps::{Gap, Reason};
        let gap = |day: i64, from: u16, to: u16, reason: Reason| Gap {
            day,
            from,
            to,
            reason,
        };
        let outage = pull::calendar::SYSTEMS_OUTAGE_DAY;
        assert_eq!(
            run_word(
                &gap(outage, 700, 710, Reason::OutsideWindow),
                false,
                None,
                &silent_reach()
            ),
            "halted"
        );
        assert_eq!(
            run_word(
                &gap(18_334, 570, 580, Reason::OutsideWindow),
                false,
                None,
                &silent_reach()
            ),
            "halt-suspected"
        );
        assert_eq!(
            run_word(
                &gap(outage, 700, 710, Reason::OutsideWindow),
                false,
                Some(outage - 1),
                &silent_reach()
            ),
            "expired"
        );
    }

    /// **AN ABSENT OR SHORT BOUND IS NAMED `null`, NOT A DATE.** Runs
    /// `gaps_body`'s `day_text` refusal for a day no calendar can name.
    #[test]
    fn a_bound_no_calendar_can_name_is_null() {
        let calendar = calendar_of(
            "record",
            (epoch_day(2025, 5, 1), i64::MAX),
            &[month(2025, 5)],
        );
        assert!(calendar["last"].is_null(), "{calendar}");
        assert_eq!(calendar["first"], "2025-05-01");
    }

    /// **A FULL VOTE MAP IS A NAMED `429`, AND A FINISHED BUILD'S REFUSAL IS
    /// ANSWERED WITH ITS OWN STATUS** — or `500` for a status no answer can
    /// carry. Runs `votes_full` and `vote_of`'s refusal arm.
    #[test]
    fn a_full_vote_map_and_a_refused_build_answer_by_name() {
        let (status, headers, body) = votes_full();
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(
            body.contains(&crate::calendar_of::build::MAX_BUILDS.to_string())
                && body.contains("calendar=record"),
            "{body}"
        );
        let refused = vote_of(Err((503, "the pool said no".to_owned()))).expect_err("refused");
        assert_eq!(refused.0, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refused.2, "the pool said no");
        let nonsense = vote_of(Err((1_000, String::new()))).expect_err("refused");
        assert_eq!(nonsense.0, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// One one-second bar `second` seconds past IST midnight of `day`.
    fn second_bar(day: i64, second: i64) -> store::format::Bar {
        store::format::Bar {
            ts_micros: (day * 86_400 - 19_800 + second) * 1_000_000,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: store::format::OI_NULL,
        }
    }

    /// **WITH NO CALENDAR GIVEN THE TYPED TABLE DECIDES, A HELD SECOND NO
    /// SESSION OWES IS OUTSIDE, AND A DAY HOLDING EVERY OWED SECOND HAS NO
    /// LONGEST RUN.** Runs `second_audit`'s table fallback and its in-session
    /// outside count, `trade_days`' table fallback, and the `null` longest run
    /// `one_second_members` draws for a whole day.
    #[test]
    fn a_whole_day_by_the_table_has_no_missing_second() {
        let day = epoch_day(2025, 5, 2);
        let mut rows: Vec<store::format::Bar> = (33_300..55_800)
            .map(|second| second_bar(day, second))
            .collect();
        rows.insert(0, second_bar(day, 30_000));
        let held = summary_of(day, &rows);
        let audit = second_audit(&held, day, day, None, 1);
        assert_eq!(
            (audit.expected, audit.held, audit.missing),
            (22_500, 22_500, 0)
        );
        assert_eq!(audit.outside, 1, "08:20:00 is held and owed by no session");
        assert_eq!(audit.days.len(), 1);
        assert_eq!(audit.days[0].longest, None);
        let members = one_second_members(
            &held,
            day,
            day,
            None,
            false,
            None,
            1,
            None,
            (0, 0),
            &silent_reach(),
        );
        assert!(members.contains(r#""longest_run":null"#), "{members}");

        let days = trade_days(&held, day, day, None, None);
        assert_eq!((days.sessions, days.held), (1, 1));
        assert!(days.without.is_empty());
    }

    /// **PAST `MAX_GAPS` LISTED RUNS THE LIST STOPS AND SAYS SO**, while every
    /// run is still counted. Runs `close_run`'s truncation.
    #[test]
    fn past_max_gaps_listed_runs_the_list_is_truncated_by_name() {
        let mut audit = SecondAudit::default();
        let mut day = SecondDay {
            day: 1,
            expected: 0,
            held: 0,
            missing: 0,
            longest: None,
        };
        let runs = u32::try_from(pull::gaps::MAX_GAPS).expect("small") + 3;
        for at in 0..runs {
            let mut open = Some((at * 10, at * 10 + 4));
            close_run(&mut audit, &mut day, &mut open, 1);
        }
        assert_eq!(audit.runs.len(), pull::gaps::MAX_GAPS);
        assert!(audit.truncated);
        assert_eq!(audit.run_count, u64::from(runs));
    }

    /// **A MALFORMED AUDIT IS A `400` NAMING WHAT IS WRONG**, and so is a
    /// symbol the store cannot name when the census is asked to find it.
    /// Runs `gaps_json`'s parse refusal and `identity_in_census`'s symbol
    /// refusal.
    #[tokio::test]
    async fn a_malformed_audit_and_an_unnamable_symbol_are_refused_by_name() {
        let root = crate::server::tests::store_root("cloud-gaps-malformed");
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters("cloud-gaps-malformed", None, None),
            &root,
        ));
        for (query, word) in [
            (
                "feed=nobody&symbol=NIFTY&timeframe=1min&month=2025-05",
                "feed",
            ),
            (
                "feed=gdfl&symbol=not%20a%20symbol&timeframe=1s&month=2025-05",
                "is not a symbol the store can name",
            ),
        ] {
            let uri: axum::http::Uri = format!("/gaps.json?{query}").parse().expect("a URI");
            let (code, _, body) = gaps_json(axum::extract::State(Loaded::clone(&site)), uri).await;
            assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{query}: {body}");
            assert!(body.contains(word), "{query}: {body}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A ONE-SECOND SHARE IS TRADE-DRIVEN AND AUDITED ON THE EXCHANGE
    /// CALENDAR, NOT THE CASH SCHEDULE** (D-3336 §2): it owes no count. Runs
    /// `audit_one`'s trade-driven cash arm.
    #[tokio::test]
    async fn a_one_second_share_owes_no_count() {
        let root = crate::server::tests::store_root("cloud-gaps-second-share");
        let day = epoch_day(2025, 5, 2);
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "CASH",
            symbol: "RELIANCE",
            contract: None,
            timeframe: store::path::Timeframe::SECOND_1,
            month: month(2025, 5),
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        let id = u32::try_from(brutex_core::universe::fnv1a("RELIANCE") & u64::from(u32::MAX))
            .expect("low 32 bits");
        let mut file = store::file::BarFile::open_or_create(&root, path, id).expect("a file");
        file.append(&[second_bar(day, 33_300)]).expect("one print");
        drop(file);
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters("cloud-gaps-second-share", None, None),
            &root,
        ));
        let uri: axum::http::Uri =
            "/gaps.json?feed=gdfl&exchange=NSE&segment=CASH&symbol=RELIANCE&timeframe=1s&month=2025-05"
                .parse()
                .expect("a URI");
        let (code, _, body) = gaps_json(axum::extract::State(site), uri).await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let audited = &body["month"][0];
        assert_eq!(audited["trade_driven"], true, "{audited}");
        assert_eq!(audited["expected"], 0);
        assert_eq!(audited["lost_minutes"], 0);
        assert_eq!(audited["days_held"], 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **EVERY WAY A VOTE JOIN CAN END IS ANSWERED BY NAME.** On one served
    /// site with no store: while the census is read at start the vote says it
    /// waits on that (`202`); a build queued for the series and never begun
    /// is followed and waited out, and says it is queued (`202`); finished
    /// under the stamp the request reads, its answer is parked and handed to
    /// the next request as it is; and with `MAX_BUILDS` other builds in
    /// flight a new one is not admitted (`429`). Runs `voted`'s parked,
    /// start-up, full and queued arms, which no test reached.
    #[tokio::test]
    async fn every_way_a_vote_join_ends_is_answered_by_name() {
        use crate::calendar_of::build::{Joined, MAX_BUILDS};
        let root = crate::scratch::path("cloud-vote-joins");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        let asked = subject("INDEX", None, store::path::Timeframe::MINUTE_1);
        let reason = |answer: Result<PeerCalendar, GapsAnswer>| {
            let Err((status, _, body)) = answer else {
                panic!("a vote with no build behind it answered");
            };
            let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
            (
                status,
                body["reason"].as_str().unwrap_or_default().to_owned(),
            )
        };
        let builds = &site.calendars.builds;
        {
            let _reading = builds.reading_census_at_start();
            let (status, why) = reason(voted(&site, &asked).await);
            assert_eq!(status, axum::http::StatusCode::ACCEPTED);
            assert!(why.contains("reading its census at start"), "{why}");
        }
        builds.bound_waits(std::time::Duration::from_millis(1));
        let now = crate::server::census_cache::manifest_stamps(&site.store_root);
        let key = vote_key(&asked);
        let Joined::Lead(queued) = builds.votes.join(&key, Some(&now), None) else {
            panic!("the first join leads");
        };
        let (status, why) = reason(voted(&site, &asked).await);
        assert_eq!(status, axum::http::StatusCode::ACCEPTED);
        assert!(why.contains("it is queued"), "{why}");
        drop(queued.enter(Some(now.clone())));
        builds
            .votes
            .flights
            .finish(&key, &queued, Err((418, "parked".to_owned())));
        let Err((status, _, body)) = voted(&site, &asked).await else {
            panic!("a parked refusal is handed out as it is");
        };
        assert_eq!((status.as_u16(), body.as_str()), (418, "parked"));
        let mut held = Vec::with_capacity(MAX_BUILDS);
        for other in 0..MAX_BUILDS {
            let mut key = key.clone();
            key.3 = format!("OTHER{other}");
            let Joined::Lead(build) = builds.votes.join(&key, Some(&now), None) else {
                panic!("room for build {other}");
            };
            held.push(build);
        }
        let Err((status, _, body)) = voted(&site, &asked).await else {
            panic!("a full build map admits no vote");
        };
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(body.contains("builds are queued or running"), "{body}");
        drop(held);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Set in the child [`a_full_calendar_pool_refuses_the_census_and_the_audit`]
    /// runs itself in, where the pool it fills is that process's alone.
    const FULL_POOL_CHILD: &str = "BRUTEX_CLOUD_MUTANT_FULL_CALENDAR_POOL_CHILD";

    /// **A FULL CALENDAR POOL REFUSES BY NAME, BOTH WHERE THE SERIES IS FOUND
    /// IN THE CENSUS AND WHERE ITS MONTHS ARE AUDITED** — `429`, never an
    /// audit that did not happen. Every slot of the pool is held; then a
    /// request that names no exchange or segment must read the census to
    /// find them, and a request that names both must audit its span, and
    /// each is refused. So is a peer vote led while the pool is full: its
    /// build is finished with the refusal. Runs `gaps_json`'s two admission
    /// refusals and `lead_vote`'s, which no test reached. The pool is
    /// process-wide, so the test runs itself in a child process where
    /// nothing else needs it.
    #[test]
    fn a_full_calendar_pool_refuses_the_census_and_the_audit() {
        if std::env::var_os(FULL_POOL_CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
                .args([
                    "--exact",
                    "server::gaps_route::cloud_mutant_tests::\
                     a_full_calendar_pool_refuses_the_census_and_the_audit",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env(FULL_POOL_CHILD, "1")
                .output()
                .expect("the child test process starts");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "child failed or ran no test:\n{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            let root = crate::scratch::path("cloud-full-calendar-pool");
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("a scratch root");
            let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
            let gate = std::sync::Arc::new(std::sync::Barrier::new(
                crate::detail::MAX_CALENDAR_CONCURRENT + 1,
            ));
            let holders: Vec<_> = (0..crate::detail::MAX_CALENDAR_CONCURRENT)
                .map(|_| {
                    let gate = std::sync::Arc::clone(&gate);
                    tokio::spawn(crate::detail::run_calendar(move || {
                        gate.wait();
                    }))
                })
                .collect();
            while crate::detail::run_calendar(|| ()).await != Err(crate::detail::RunError::Saturated)
            {
                tokio::task::yield_now().await;
            }
            for query in [
                "/gaps.json?feed=dhan&symbol=NIFTY&timeframe=1min&month=2025-05",
                "/gaps.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2025-05",
            ] {
                let uri: axum::http::Uri = query.parse().expect("a URI");
                let (status, _, body) = gaps_json(axum::extract::State(site.clone()), uri).await;
                assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{query}: {body}");
                assert!(body.contains("calendar derivation not admitted"), "{query}: {body}");
            }
            // A PEER VOTE LED NOW is refused where its reading is spawned, and
            // the build is finished with that refusal, never left unanswered.
            let asked = subject("INDEX", None, store::path::Timeframe::MINUTE_1);
            let Err((status, _, body)) = voted(&site, &asked).await else {
                panic!("a vote whose reading was not admitted answered");
            };
            assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{body}");
            assert!(body.contains("calendar derivation not admitted"), "{body}");
            gate.wait();
            for holder in holders {
                holder.await.expect("a holder").expect("admitted");
            }
            let _ = std::fs::remove_dir_all(&root);
        });
    }

    /// **`gaps_answer` IS THE ROUTE'S OWN ANSWER, AND A QUERY THAT IS NO URI
    /// IS A NAMED `400`.** The door the gate-8 bench times (R3-06) had no
    /// caller in any test.
    #[tokio::test]
    async fn the_bench_door_answers_as_the_route_and_refuses_what_is_no_uri() {
        let root = crate::server::tests::store_root("cloud-gaps-door");
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters("cloud-gaps-door", None, None),
            &root,
        ));
        let query =
            "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2026-08";
        let uri: axum::http::Uri = format!("/gaps.json?{query}").parse().expect("a URI");
        let (status, _, said) = gaps_json(axum::extract::State(site.clone()), uri).await;
        assert_eq!(
            gaps_answer(site.clone(), query).await,
            (status.as_u16(), said)
        );
        let (refused, body) = gaps_answer(site, "feed=gdfl month 2026-08").await;
        assert_eq!(refused, 400, "{body}");
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert!(
            body["error"].as_str().is_some_and(|why| !why.is_empty()),
            "the refusal names its reason: {body}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **ONLY AN NSE SHARE WITH NO CONTRACT IS AUDITED AGAINST THE DATED CASH
    /// TABLE.** The same symbol on BSE, or named with a contract below it,
    /// counts against the calendar of record. Takes the false branches of
    /// `exchange == "NSE"` and `contract.is_none()` in the route's cash test,
    /// and of `contract.is_none()` in `audit_one`'s classifier choice.
    #[tokio::test]
    async fn only_an_nse_share_with_no_contract_counts_against_the_cash_table() {
        let root = crate::server::tests::store_root("cloud-gaps-cash-only");
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters("cloud-gaps-cash-only", None, None),
            &root,
        ));
        let source = |query: &'static str| {
            let site = site.clone();
            async move {
                let uri: axum::http::Uri = format!(
                    "/gaps.json?feed=zerodha&segment=CASH&symbol=RELIANCE&timeframe=1min\
                     &month=2026-08&{query}"
                )
                .parse()
                .expect("a URI");
                let (status, _, body) = gaps_json(axum::extract::State(site), uri).await;
                assert_eq!(status, axum::http::StatusCode::OK, "{body}");
                let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
                body["calendar"]["source"].clone()
            }
        };
        assert_eq!(source("exchange=NSE").await, "table");
        assert_eq!(source("exchange=BSE").await, "record");
        assert_eq!(
            source("exchange=NSE&contract=2026-08-27-2500000-CE").await,
            "record"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A RUN THAT REACHES PAST A RECORDED HALT IS NOT CALLED HALTED**: the
    /// halt must hold both its ends. Takes the false branch of
    /// `halt.halted.holds(gap.to)` in `run_word`.
    #[test]
    fn a_run_reaching_past_a_halt_is_not_called_halted() {
        use pull::gaps::{Gap, Reason};
        let outage = pull::calendar::SYSTEMS_OUTAGE_DAY;
        let run = |to: u16| Gap {
            day: outage,
            from: 700,
            to,
            reason: Reason::OutsideWindow,
        };
        assert_eq!(run_word(&run(710), false, None, &silent_reach()), "halted");
        assert_eq!(
            run_word(&run(1_000), false, None, &silent_reach()),
            "outside-window"
        );
    }

    /// Records one month of `symbol`, a spot index on GDFL at `rung`, in GDFL's
    /// census, as a landing would.
    fn census_holds_index(
        root: &std::path::Path,
        symbol: &str,
        rung: store::path::Timeframe,
        month: YearMonth,
    ) {
        let path = pull::manifest::manifest_path(root, Vendor::Gdfl);
        let bytes = std::fs::read(&path).unwrap_or_default();
        let mut manifest =
            pull::manifest::Manifest::open_image(Vendor::Gdfl, &bytes).expect("a census");
        manifest
            .record(pull::manifest::Entry {
                key: pull::manifest::EntryKey {
                    contract: None,
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol: brutex_core::symbol::Symbol::new(symbol).expect("a symbol"),
                    timeframe: rung,
                    month,
                },
                rows: 1,
                first_ts_micros: 1,
                last_ts_micros: 2,
            })
            .expect("one month in the census");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the census directory");
        std::fs::write(&path, manifest.image()).expect("publish the census");
    }

    /// **A PEER THAT HOLDS NO SESSION IS NO VOTE, AND NO UNREADABLE PEER
    /// EITHER.** BANKNIFTY's census names its daily and minute months, and
    /// both files open and hold no bar: the vote has no calendar and names
    /// nobody, as voter or as unreadable. One daily bar makes the same peer a
    /// voter. Takes the false branch of `calendar.sessions() > 0` in
    /// `peer_vote_reading`.
    #[test]
    fn a_peer_that_holds_no_session_is_no_vote() {
        let root = crate::server::tests::store_root("cloud-gaps-empty-peer");
        let masters = crate::server::tests::masters("cloud-gaps-empty-peer", None, None);
        let held = month(2026, 8);
        let id = u32::try_from(brutex_core::universe::fnv1a("BANKNIFTY") & u64::from(u32::MAX))
            .expect("low 32 bits");
        let paths = [
            store::path::Timeframe::DAY_1,
            store::path::Timeframe::MINUTE_1,
        ]
        .map(|rung| {
            census_holds_index(&root, "BANKNIFTY", rung, held);
            let path = store::path::StorePath::new(store::path::PathParts {
                vendor: Vendor::Gdfl,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "BANKNIFTY",
                contract: None,
                timeframe: rung,
                month: held,
                file: store::path::FileKind::Bars,
            })
            .expect("a legal path");
            // CREATED AND CLOSED: a writer holds its month locked, and a
            // locked month is a peer that would not open.
            drop(store::file::BarFile::open_or_create(&root, path, id).expect("an empty month"));
            path
        });
        let asked = subject("INDEX", None, store::path::Timeframe::MINUTE_1);

        let (_, vote) = peer_vote_reading(&Site::load(&masters, &root), &asked, census_now_stamped);
        assert!(vote.calendar.is_none(), "an empty peer agrees on nothing");
        assert!(vote.from.is_empty(), "{:?}", vote.from);
        assert!(vote.unreadable.is_empty(), "{:?}", vote.unreadable);

        let tuesday = epoch_day(2026, 8, 4);
        let mut daily =
            store::file::BarFile::open_or_create(&root, paths[0], id).expect("the daily month");
        daily
            .append(&[store::format::Bar {
                ts_micros: (tuesday * 86_400 - 19_800) * 1_000_000,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 0,
                open_interest: i64::MIN,
            }])
            .expect("one daily bar");
        drop(daily);
        let (_, vote) = peer_vote_reading(&Site::load(&masters, &root), &asked, census_now_stamped);
        assert_eq!(vote.from, vec!["gdfl:BANKNIFTY".to_owned()]);
        assert!(vote.calendar.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// One one-second NIFTY month on GDFL, 2025-05: `count` bars a second
    /// apart from 09:15:00 on the 2nd, the first two each carrying `volume`.
    /// Returns the bar file's path.
    fn one_second_index(root: &std::path::Path, count: i64, volume: i64) -> std::path::PathBuf {
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::SECOND_1,
            month: month(2025, 5),
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        let id = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
            .expect("low 32 bits");
        let mut file = store::file::BarFile::open_or_create(root, path, id).expect("a file");
        let day = epoch_day(2025, 5, 2);
        let rows: Vec<store::format::Bar> = (0..count)
            .map(|n| store::format::Bar {
                volume: if n < 2 { volume } else { 0 },
                ..second_bar(day, 33_300 + n)
            })
            .collect();
        file.append(&rows).expect("generated records");
        path.to_path_buf(root)
    }

    /// `/gaps.json` for the one-second NIFTY month of 2025-05 on GDFL: the
    /// month's object.
    async fn one_second_audit(root: &std::path::Path, tag: &str) -> serde_json::Value {
        let site = std::sync::Arc::new(Site::serving(
            &crate::server::tests::masters(tag, None, None),
            root,
        ));
        let uri: axum::http::Uri =
            "/gaps.json?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2025-05"
                .parse()
                .expect("a URI");
        let (status, _, body) = gaps_json(axum::extract::State(site), uri).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        body["month"][0].clone()
    }

    /// **A ONE-SECOND MONTH WHOSE ROWS THE FOLD REFUSES IS NAMED IN THE
    /// FOLD'S OWN WORDS AND CLAIMS NOTHING**: two prints in one minute whose
    /// volumes sum past `i64::MAX`. The kept summary saw the sum saturate, so
    /// the bars are read, the fold refuses them, and the month owes and holds
    /// no minute. Takes the true branch of `extremes.volume == i64::MAX` in
    /// `month_input`, its fallback sentence, and every `fold_error` arm of
    /// `audit_one`.
    #[tokio::test]
    async fn a_month_the_fold_refuses_is_named_and_claims_nothing() {
        let root = crate::server::tests::store_root("cloud-gaps-fold-refused");
        one_second_index(&root, 1_100, i64::MAX / 2 + 1);
        let audited = one_second_audit(&root, "cloud-gaps-fold-refused").await;
        assert!(
            audited["fold_error"]
                .as_str()
                .is_some_and(|why| why.contains("VolumeOverflow")),
            "{audited}"
        );
        assert_eq!(audited["summary"], "built", "{audited}");
        assert_eq!(
            audited["summary_fallback"],
            "the month holds rows the fold may refuse; its answer is read from the bars"
        );
        assert_eq!(
            (audited["expected"].as_u64(), audited["held"].as_u64()),
            (Some(0), Some(0))
        );
        assert_eq!(audited["gaps"], serde_json::json!([]), "no run is claimed");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A KEPT SUMMARY THAT COUNTS ANY ROW THE FOLD MAY REFUSE SENDS THE
    /// AUDIT TO THE BARS, WHATEVER THE KIND.** The month's `.sum` is
    /// rewritten with one count raised at a time (a decreasing stamp, a
    /// negative volume, a row outside the month), as a summary written by a
    /// writer that admitted such a row would say. Each is read from the bars,
    /// named, and — the bars themselves being clean — audited whole. Takes
    /// the true branch of each of the three counts in `month_input`.
    #[tokio::test]
    async fn a_kept_summary_counting_any_row_the_fold_may_refuse_reads_the_bars() {
        let root = crate::server::tests::store_root("cloud-gaps-kept-refusable");
        let bars = one_second_index(&root, 1_100, 0);
        let clean = one_second_audit(&root, "cloud-gaps-kept-refusable").await;
        assert_eq!(clean["summary"], "kept", "{clean}");
        let kept =
            store::month_summary::read_kept(&bars, 1_100).expect("the writer kept a summary");
        let raise: [fn(&mut store::month_summary::MonthSummary); 3] = [
            |summary| summary.counts.decreasing = 1,
            |summary| summary.counts.negative_volume = 1,
            |summary| summary.counts.outside_month = 1,
        ];
        for one in raise {
            let mut forged = kept.clone();
            one(&mut forged);
            std::fs::write(
                store::month_summary::path_beside(&bars),
                forged.file_image(),
            )
            .expect("the rewritten summary");
            let audited = one_second_audit(&root, "cloud-gaps-kept-refusable").await;
            assert_eq!(audited["summary"], "built", "{audited}");
            assert_eq!(
                audited["summary_fallback"],
                "the month holds rows the fold may refuse; its answer is read from the bars"
            );
            assert_eq!(audited["fold_error"], serde_json::Value::Null, "{audited}");
            assert_eq!(audited["held"], clean["held"], "the same rows, read");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A RUN OF MISSING SECONDS ON A DAY AFTER TODAY IS `not-yet`, NEVER A
    /// VENDOR HOLE**: the session has not happened. The same month audited
    /// with a clock that has passed the day calls the run a vendor hole.
    /// Takes the true branch of `reach.not_yet(run.day)` in
    /// `one_second_members`.
    #[test]
    fn a_missing_second_after_today_is_not_yet() {
        let day = epoch_day(2025, 5, 2);
        let rows: Vec<store::format::Bar> = (33_300..33_400)
            .filter(|second| !(33_350..33_360).contains(second))
            .map(|second| second_bar(day, second))
            .collect();
        let held = summary_of(day, &rows);
        let words = |today: i64| {
            let reach = super::super::feeds_route::ImportReach {
                today: Some(today),
                ..silent_reach()
            };
            let members =
                one_second_members(&held, day, day, None, false, None, 10, None, (0, 0), &reach);
            let members: serde_json::Value =
                serde_json::from_str(&format!("{{{}}}", &members[1..])).expect("members");
            members["second_runs"]
                .as_array()
                .expect("runs")
                .iter()
                .filter(|run| run["from_second"] == 33_350)
                .map(|run| run["reason"].as_str().expect("a word").to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(words(day - 1), vec![NOT_YET_WORD.to_owned()]);
        assert_eq!(words(day + 1), vec!["vendor-hole".to_owned()]);
    }
}
