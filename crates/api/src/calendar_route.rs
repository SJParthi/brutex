//! `/calendar.json` -- the calendar of record per feed and rung, held by the
//! feed's stored series, and the refusals a calendar-deriving route answers
//! with.
//!
//! # Where it lives
//!
//! Cut out of `server.rs` by the WP-01 seam and mounted there as a child
//! module, so it reads that file's private helpers through `super::` and
//! `server.rs` imports each name back.
//!
//! # What it answers since D-3336 (WP-35)
//!
//! It used to be a vote: every spot series the feed held derived its own
//! calendar off its bars, and the union was the exchange's. That was 343
//! derivations on the operator's Zerodha store, 6.7 s to 36.9 s cold for 4.8
//! to 7.4 MB, re-derived at every landing; it held 74 Saturdays and 71
//! Sundays, 132 of them voted by `NIFTYGS10YR` alone, so 2015 read 357
//! sessions; and GDFL, which holds only `1s` files, answered
//! `{"sessions":0,"days":[]}` (DB-015, DB-G29, DB-G48, DB-G85).
//!
//! Now each day's status is the calendar of record's
//! ([`crate::calendar_of::record`]), and the feed's bars say only what it
//! HOLDS: NIFTY and BANKNIFTY for the exchange, or the series asked for, at
//! the rung asked for (`1s` included) or the feed's default. The answer is a
//! dense day-status string, built once per feed generation and kept under
//! the feed's manifest stamp; a request that finds it is one map probe.
//!
//! Measured on a shadow of the operator's store, 9 Oct 2026, release build,
//! sequential GETs: a kept answer p50 0.41 ms, p99 0.59 ms (n=50) for 18.9 KB
//! (Zerodha) or 17.0 KB (GDFL); a build after a Zerodha landing, once the
//! census is read, p50 8.5 ms, p99 20.0 ms (n=20); a first build 0.12 s
//! (Zerodha, 282 daily months) and 3.6 s (GDFL, 194 one-second months, 142
//! of them without a `.tix` and so bisected). The census re-read a landing
//! forces is shared by every census route and is not this route's: about
//! 0.62 s on that store (WP-30 keys it per vendor).
//!
//! # The first request after a start (WP-36, DB-N19)
//!
//! That first build was paid by whoever asked first: 6.55 s for
//! `?feed=gdfl&symbol=NIFTY` on the main session's shadow, 3.3-5.1 s on
//! WP-36's. Now a request is handed a confirmed answer without reading the
//! census, or joins a build and waits for it at most a budget, past which it
//! is answered `202` `"building"` with the build's progress; and the served
//! process reads the census and builds the answers the pages ask first at
//! start ([`warm_at_start`]). See [`crate::calendar_of::build`]. Measured on
//! WP-36's shadow, release build, 10 Oct 2026: the first
//! `?feed=gdfl&symbol=NIFTY` after five restarts answered `202` in 1.9-2.4 ms
//! and in full 1.1-3.7 s later; a confirmed answer, n=200 sequential, p50
//! 0.43-0.47 ms, p99 0.53-0.83 ms, max 0.56-1.41 ms over eight keys; every
//! answer byte for byte the build before WP-36's.

use super::{
    CensusNow, CensusStamps, Loaded, Site, census_now_stamped, no_such_feed_json, param,
    stored_case,
};
use crate::{census, ingest};
use brutex_core::vendor::Vendor;

/// The identity a calendar is derived for — everything the store path needs.
///
/// Named because it is a `HashMap` key, a sort key and a destructuring target,
/// and the bare triple reads as a puzzle by its third appearance.
pub(super) type SpotIdentity = (
    brutex_core::instrument::Exchange,
    brutex_core::instrument::Segment,
    brutex_core::symbol::Symbol,
);

/// Every SPOT series a census reading holds, grouped by its whole identity.
///
/// # Why this is a function rather than six lines in the loop
///
/// So it can be tested. The defect it exists to prevent lived in exactly this
/// grouping: the map was keyed on `series.symbol` alone, and the caller then
/// had nothing left to pass for `exchange` and `segment`, so it substituted the
/// literals `"NSE"` and `"INDEX"` — for every instrument, whatever the census
/// said. `ADANIENT` is stored at `NSE/CASH/ADANIENT/`, so every probe went to
/// `NSE/INDEX/ADANIENT/` and missed: **366 measured refusals** against 1,240
/// daily bars that were on disk the whole time.
///
/// A test over `derive` cannot catch that — `derive` always honoured the
/// segment it was handed. The fault was in what the caller handed it, and this
/// is the smallest piece of the caller that can be given a census and asked
/// what it kept.
///
/// **Contract-bearing series are dropped, deliberately.**
/// [`crate::calendar_of::derive`] addresses a series by exchange, segment and
/// symbol and hands [`crate::bars::open`] a `contract: None`, so an option or
/// future CANNOT be read through it — those bars live at `symbol/contract/` and
/// the probe would open `symbol/`. That is not a narrowing of the answer: a
/// calendar is which DAYS a venue traded, and every contract on a venue trades
/// that venue's days. Reading one through a spot path returns an empty calendar
/// that votes for nothing, which is the silent miss this whole change ends.
pub(super) fn spot_months_by_identity(
    held: &[(census::Series, store::path::YearMonth)],
) -> std::collections::HashMap<SpotIdentity, Vec<store::path::YearMonth>> {
    let mut by_series: std::collections::HashMap<SpotIdentity, Vec<store::path::YearMonth>> =
        std::collections::HashMap::with_capacity(held.len());
    for (series, month) in held {
        if series.contract.is_some() {
            continue;
        }
        by_series
            .entry((series.exchange, series.segment, series.symbol))
            .or_default()
            .push(*month);
    }
    // ONE ENTRY PER MONTH, NOT ONE PER RUNG.
    //
    // `held_entries` yields `(Series, YearMonth)` and `Series` carries the
    // TIMEFRAME, so a month held at nine rungs arrives nine times. This map
    // drops the timeframe — deliberately, because `derive` probes the daily and
    // minute rungs itself — so without this the same month is handed to it
    // nine times and probed nine times.
    //
    // MEASURED on the operator's store during a live repull: **4,446**
    // `api.bars read refused` warnings from one page, against a store holding
    // 61 months. Every one was a real file probe that opened nothing, and every
    // one wrote a `Warn` line. Sorting first is what makes `dedup` correct —
    // it only removes ADJACENT equals — and it also fixes the order, which the
    // `HashMap` walk does not give.
    for months in by_series.values_mut() {
        months.sort_unstable();
        months.dedup();
    }
    by_series
}

/// One `/calendar.json` answer: status, content type, body.
type CalendarAnswer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// The answer `/calendar.json` gives a feed whose census could not be read.
///
/// # Why a refusal, and not the empty calendar it used to be
///
/// An unreadable census contributes no entry, so both branches below found no
/// series and answered `200 {"sessions":0,"days":[]}` -- byte for byte the
/// answer for a feed that holds nothing. The ingest page reads that as "the
/// store holds no bars for this feed", which is a claim about the store made
/// from a fact about its counter: the fallback that hides a failure `CLAUDE.md`
/// §4 bans, and the one D-0124 already refused on `/store.json`. So the answer
/// is 503, the state is named, and the census's own note says which file and
/// why. An ABSENT census is still the ordinary state and still answers 200.
///
/// `own` is the asked feed's census, found once by `calendar_json` for this
/// and for `census_holds`, so the route searches the census rows once.
fn unreadable_calendar(own: Option<&census::VendorCensus>, feed: Vendor) -> Option<CalendarAnswer> {
    let damaged = own.filter(|census| matches!(census.state, census::Census::Unreadable { .. }))?;
    Some((
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "status": "refused",
            "feed": feed.as_str(),
            "census": "unreadable",
            "refusal": format!(
                "{}. This feed's census could not be read, so which series it holds \
                 is unknown and no calendar was derived; an empty one would claim \
                 the store holds no bars.",
                damaged.note()
            ),
        })
        .to_string(),
    ))
}

/// The answer `/calendar.json` gives when a bar file this feed's census holds
/// could not be opened for the derivation.
///
/// # Why a refusal, and why it is not kept
///
/// `calendar_of::derive` names a month it could not open and derives the
/// calendar without it, and `cached` used to throw that report away. So a bar
/// directory gone for one request answered `200 {"sessions":0}` -- the empty
/// calendar `unreadable_calendar` refuses for an unreadable census -- and the
/// days a month that did not open would have proved were closed days in any
/// calendar that spanned it. The census says the file is there, so the store
/// disagrees with its own counter for this read. The answer is 503, the first
/// refusal is quoted, and the count says how many there were: a store root gone
/// for the request refuses every held month, and naming each would grow the
/// body with the store. `cached` keeps no such derivation, so the next request
/// derives again. D-0695.
fn unopened_calendar(feed: Vendor, symbol: &str, unopened: &[String]) -> CalendarAnswer {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "status": "refused",
            "feed": feed.as_str(),
            "symbol": symbol,
            "bars": "unopened",
            "unopened": unopened.len(),
            "refusal": format!(
                "{} bar file(s) this feed's census holds for {symbol} could not be \
                 opened, the first: {}. The days they would prove are unknown, so no \
                 calendar is answered; one derived without them would put a holiday \
                 in the calendar that never happened.",
                unopened.len(),
                unopened.first().map_or("", String::as_str),
            ),
        })
        .to_string(),
    )
}

fn calendar_conflict_json(
    feed: Vendor,
    symbol: &str,
    first: (
        brutex_core::instrument::Exchange,
        brutex_core::instrument::Segment,
    ),
    second: (
        brutex_core::instrument::Exchange,
        brutex_core::instrument::Segment,
    ),
) -> String {
    serde_json::json!({
        "status": "refused",
        "feed": feed.as_str(),
        "symbol": symbol,
        "refusal": format!(
            "ambiguous stored symbol {symbol}: {}/{} and {}/{} are different calendar identities; no calendar was selected",
            first.0.as_str(), first.1.as_str(), second.0.as_str(), second.1.as_str()
        ),
    }).to_string()
}

pub(super) async fn calendar_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> CalendarAnswer {
    // THE SPAN ASKED (DB-B42): `held` (the default) is the span the feed's
    // bars reach; `record` widens a held answer to the calendar of record's
    // whole span, so /ingest draws 2026-10-02 closed beyond GDFL's last held
    // day instead of guessing a weekday. Any other word is refused.
    let query = uri.query().unwrap_or("");
    let span = param(query, "span");
    if !matches!(span.as_str(), "" | "held" | "record") {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )],
            serde_json::json!({
                "error": format!(
                    "span={span:?} is not a span this route answers: held (the default, the \
                     days the feed's bars reach) or record (the calendar of record's whole span)"
                )
            })
            .to_string(),
        );
    }
    let rung = rung_param(query).ok().flatten();
    let answer = calendar_json_held(site, uri).await;
    if span == "record"
        && answer.0 == axum::http::StatusCode::OK
        && let Some(widened) = crate::calendar_of::record::widen_to_record(&answer.2, rung)
    {
        return (answer.0, answer.1, widened);
    }
    answer
}

/// [`calendar_json`] over the span the feed holds: the answer every caller
/// got before `span` existed.
async fn calendar_json_held(site: Loaded, uri: axum::http::Uri) -> CalendarAnswer {
    use crate::calendar_of::build::Joined;
    // THE QUESTION FIRST: a malformed one is refused before anything is read.
    let asked = match Asked::of(&uri) {
        Ok(asked) => asked,
        Err(refused) => return refused,
    };
    // A CONFIRMED ANSWER IS HANDED OUT WITHOUT READING THE CENSUS (DB-N19):
    // the asked feed's stamp, one probe, and the body's copy, on this worker.
    // It was confirmed under that very stamp when it was kept or found kept,
    // so it is the answer the reading below would give. See
    // `crate::calendar_of::build`.
    let now = feed_stamps(&site.store_root, asked.feed);
    if let Some(kept) = site.calendars.builds.confirmed(&asked.key, &now) {
        return answered(&kept);
    }
    // ANYTHING ELSE IS A BUILD, joined or led, and waited for at most the
    // served budget. The reading itself is what it always was: on the
    // calendar pool behind its admission (W1-api2-11, D-1443), the census on
    // the manifests' stamps (D-0686, D-0695), and the answer kept under the
    // feed's (D-3336). What changed is that the request no longer pays a
    // whole build: past the budget it is answered "building", and the build
    // goes on (DB-N19).
    let build = match site.calendars.builds.join(&asked.key, Some(&now)) {
        Joined::Parked(answer) => return answered(&answer),
        Joined::CensusAtStart(since) => return building_census(&asked, since),
        Joined::Full => return builds_full(),
        Joined::Follow(build) => build,
        Joined::Lead(build) => {
            lead(&site, uri, &asked, &build, now);
            build
        }
    };
    // A START-UP BUILD STILL QUEUED BEHIND OTHERS will not be done within any
    // budget, so a request that finds one is answered at once rather than
    // after it. A request's own build is waited for: it starts at once.
    let queued_at_start = {
        let progress = build.progress();
        progress.origin == crate::calendar_of::build::Origin::StartUp
            && progress.reading_for.is_none()
    };
    let budget = site.calendars.builds.budget().map(|budget| {
        if queued_at_start {
            std::time::Duration::ZERO
        } else {
            budget
        }
    });
    match build.wait(budget).await {
        Some(answer) => answered(&answer),
        None => building(&asked, &build.progress()),
    }
}

/// Run `build`, the build a request for `asked` leads, and finish it with its
/// answer -- whatever answer: a refusal and an admission refusal finish it too,
/// so nobody waits on a build that will never answer.
///
/// OFF THE ASYNC WORKERS, AND ADMITTED. This ran inline: a census read on a
/// moved stamp and, on a miss, a derivation per spot series (0.28 s each,
/// documented) blocked a Tokio worker, and a flood or a post-pull herd blocked
/// them all. It runs on the blocking pool behind its own bounded admission.
/// Saturation answers 429 with a reason rather than queueing without bound.
/// W1-api2-11, D-1443. Since D-3336 a miss reads only the months whose census
/// entry moved (`calendar_of::month_days`). Spawned, so the build outlives a
/// request that stopped waiting for it (DB-N19).
fn lead(
    site: &Loaded,
    uri: axum::http::Uri,
    asked: &Asked,
    build: &std::sync::Arc<crate::calendar_of::build::Build>,
    now: CensusStamps,
) {
    let (site, key, build) = (
        Loaded::clone(site),
        asked.key.clone(),
        std::sync::Arc::clone(build),
    );
    drop(tokio::spawn(async move {
        let reading = {
            let (site, build) = (Loaded::clone(&site), std::sync::Arc::clone(&build));
            crate::detail::run_calendar(move || {
                let _entered = build.enter(Some(now));
                calendar_json_reading(&site, &uri, census_now_stamped)
            })
        };
        let answer = match reading.await {
            Ok(answer) => answer,
            Err(why) => calendar_admission_refused(&why),
        };
        site.calendars.builds.finish(&key, &build, kept_of(answer));
    }));
}

/// One request's question: the feed, the rung asked for and the symbol, and
/// the key its answer is kept under.
#[derive(Debug, Clone)]
pub(super) struct Asked {
    /// The feed asked for.
    feed: Vendor,
    /// The rung asked for, or `None` for the feed's default.
    rung: Option<store::path::Timeframe>,
    /// The symbol asked for, in the case the store keeps it; empty for the
    /// exchange.
    symbol: String,
    /// `(feed, rung, symbol)`.
    key: crate::calendar_of::AnswerKey,
}

impl Asked {
    /// Read the question off `uri`.
    ///
    /// # Errors
    ///
    /// The `400` a feed this build cannot read, or a rung it has no directory
    /// for, is answered.
    fn of(uri: &axum::http::Uri) -> Result<Self, CalendarAnswer> {
        let json = "application/json; charset=utf-8";
        let query = uri.query().unwrap_or("");
        let named = param(query, "feed");
        let Some(feed) = ingest::parse_vendor(&named) else {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                [(axum::http::header::CONTENT_TYPE, json)],
                no_such_feed_json(&named),
            ));
        };
        let rung = rung_param(query).map_err(|why| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                [(axum::http::header::CONTENT_TYPE, json)],
                serde_json::json!({ "error": why }).to_string(),
            )
        })?;
        let symbol = stored_case(param(query, "symbol"));
        Ok(Self {
            feed,
            rung,
            key: (feed, rung, symbol.clone()),
            symbol,
        })
    }
}

/// The asked feed's census stamp right now, and nothing else: what a
/// confirmed answer is handed out under ([`crate::calendar_of::build`]).
///
/// `manifest_stamps`' five `stat` calls, narrowed to the feed's own: an
/// answer is read from that feed's census and bar files alone, and a landing
/// of another feed moves only that feed's manifest. No manifest is read.
pub(crate) fn feed_stamps(store_root: &std::path::Path, feed: Vendor) -> CensusStamps {
    narrowed(&super::census_cache::manifest_stamps(store_root), feed)
}

/// `stamps` narrowed to `feed`'s manifest and the root.
fn narrowed(stamps: &CensusStamps, feed: Vendor) -> CensusStamps {
    CensusStamps {
        manifests: stamps
            .manifests
            .get(feed as usize)
            .copied()
            .into_iter()
            .collect(),
        root_is_dir: stamps.root_is_dir,
    }
}

/// A kept or built answer as the route answers it.
fn answered(kept: &crate::calendar_of::KeptAnswer) -> CalendarAnswer {
    (
        axum::http::StatusCode::from_u16(kept.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        kept.body.clone(),
    )
}

/// An answer as a build keeps it. Every answer this route gives carries the
/// one JSON content type, so the status and the body are the whole of it.
fn kept_of(answer: CalendarAnswer) -> crate::calendar_of::KeptAnswer {
    let (status, _, body) = answer;
    crate::calendar_of::KeptAnswer {
        status: status.as_u16(),
        body,
    }
}

/// Milliseconds in `elapsed`, saturating.
fn millis(elapsed: std::time::Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// The `202` a request is answered while its answer is being built: what the
/// build is doing, how far it has got, and when to ask again. No day is
/// claimed, so nothing here can be read as a calendar (DB-N19).
fn building(asked: &Asked, progress: &crate::calendar_of::build::Progress) -> CalendarAnswer {
    use crate::calendar_of::build::RETRY_AFTER_MS;
    let phase = if progress.reading_for.is_some() {
        "counting"
    } else {
        "queued"
    };
    let doing = match progress.reading_for {
        Some(_) if progress.owed > 0 => format!(
            "it has counted {} of the {} month file(s) it reads so far",
            progress.counted, progress.owed
        ),
        Some(_) => "it is reading the census".to_owned(),
        None => "it is queued behind the builds before it".to_owned(),
    };
    building_answer(
        asked,
        &serde_json::json!({
            "phase": phase,
            "by": progress.origin.word(),
            "since_ms": millis(progress.queued_for),
            "reading_ms": progress.reading_for.map(millis),
            "months_counted": progress.counted,
            "months_owed": progress.owed,
            "reason": format!(
                "This calendar is being built off the request path ({} build): {doing}. \
                 No day is claimed until it is whole; ask again in {RETRY_AFTER_MS} ms.",
                progress.origin.word()
            ),
        }),
    )
}

/// The `202` a request is answered while the served process reads its census
/// at start and no build for its key is queued: it does not read the census
/// beside it.
fn building_census(asked: &Asked, since: std::time::Duration) -> CalendarAnswer {
    use crate::calendar_of::build::RETRY_AFTER_MS;
    building_answer(
        asked,
        &serde_json::json!({
            "phase": "census",
            "by": "start-up",
            "since_ms": millis(since),
            "reading_ms": millis(since),
            "months_counted": 0,
            "months_owed": 0,
            "reason": format!(
                "The server is reading its census at start, which every calendar is built \
                 from; this one is built once it is read. No day is claimed; ask again in \
                 {RETRY_AFTER_MS} ms."
            ),
        }),
    )
}

/// A `"building"` answer: the asked question, `fields`, and the retry hint.
fn building_answer(asked: &Asked, fields: &serde_json::Value) -> CalendarAnswer {
    let mut body = serde_json::json!({
        "status": "building",
        "feed": asked.feed.as_str(),
        "rung": asked.rung.map(store::path::Timeframe::as_str),
        "symbol": asked.symbol,
        "retry_after_ms": crate::calendar_of::build::RETRY_AFTER_MS,
    });
    if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
        body.extend(fields.clone());
    }
    (
        axum::http::StatusCode::ACCEPTED,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body.to_string(),
    )
}

/// The `429` a request is answered when every build the map holds is queued
/// or running.
pub(super) fn builds_full() -> CalendarAnswer {
    (
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "error": format!(
                "calendar build not admitted: {} builds are queued or running, the most \
                 this server holds at once; retry",
                crate::calendar_of::build::MAX_BUILDS
            )
        })
        .to_string(),
    )
}

/// The answer a calendar-deriving route gives when its blocking work was not
/// admitted or could not be joined. W1-api2-11, D-1443.
pub(super) fn calendar_admission_refused(why: &crate::detail::RunError) -> CalendarAnswer {
    let status = if matches!(why, crate::detail::RunError::Saturated) {
        axum::http::StatusCode::TOO_MANY_REQUESTS
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        serde_json::json!({
            "error": format!(
                "calendar derivation not admitted ({why:?}): at most {} run at once, \
                 off the async workers; retry",
                crate::detail::MAX_CALENDAR_CONCURRENT
            )
        })
        .to_string(),
    )
}

/// The two spot indices the engine sweeps (`CLAUDE.md` §1): the only series
/// whose bars attest a day outside the calendar of record, and the series a
/// feed's exchange calendar is held by. D-3336.
///
/// A whitelist, not a blacklist of G-sec and bond indices: a name rule would
/// be a guess about which indices publish on a day the equity market is shut,
/// and `NIFTYGS10YR` alone voting 132 weekend days of 2015 and 2016 is what a vote
/// open to every index produced (DB-015).
pub(super) const REFERENCES: [&str; 2] = ["NIFTY", "BANKNIFTY"];

/// The rung a request names, as `rung` or `timeframe`, or `None` for the feed's
/// default.
///
/// # Errors
///
/// The refusal sentence: a rung the store has no directory for, or `rung` and
/// `timeframe` naming two different rungs.
fn rung_param(query: &str) -> Result<Option<store::path::Timeframe>, String> {
    let parse = |raw: &str| {
        store::path::Timeframe::KNOWN
            .iter()
            .copied()
            .find(|rung| rung.as_str() == raw)
            .ok_or_else(|| {
                format!(
                    "{raw:?} is not a rung this store has a directory for. It holds {}.",
                    store::path::Timeframe::KNOWN
                        .iter()
                        .map(|rung| rung.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    };
    let (rung, timeframe) = (param(query, "rung"), param(query, "timeframe"));
    match (rung.is_empty(), timeframe.is_empty()) {
        (true, true) => Ok(None),
        (false, true) => parse(&rung).map(Some),
        (true, false) => parse(&timeframe).map(Some),
        (false, false) if rung == timeframe => parse(&rung).map(Some),
        (false, false) => Err(format!(
            "rung={rung:?} and timeframe={timeframe:?} name two rungs; name one"
        )),
    }
}

/// One series' identity, and the months its census entries hold per rung.
type HeldMonths = std::collections::BTreeMap<store::path::Timeframe, Vec<store::path::YearMonth>>;

/// Every spot series of `manifest` that is a reference index or is named
/// `symbol`, with its months per rung.
///
/// One pass over the census's keys, no sort of the whole census and no copy
/// of it: the months of the few series kept are sorted afterwards. This ran
/// `census::held_entries` on every request, which builds and sorts every entry
/// the feed holds (W1-api5-5): about 1.5 s on GDFL's census. It now runs once
/// per feed generation, on a miss.
fn held_series(
    manifest: &census::Manifests,
    symbol: &str,
) -> std::collections::BTreeMap<SpotIdentity, HeldMonths> {
    let mut held: std::collections::BTreeMap<SpotIdentity, HeldMonths> =
        std::collections::BTreeMap::new();
    for key in manifest.held_keys() {
        if key.contract.is_some() {
            continue;
        }
        let name = key.symbol.as_str();
        let reference = key.exchange == brutex_core::instrument::Exchange::Nse
            && key.segment == brutex_core::instrument::Segment::Index
            && REFERENCES.contains(&name);
        if !reference && (symbol.is_empty() || name != symbol) {
            continue;
        }
        held.entry((key.exchange, key.segment, key.symbol))
            .or_default()
            .entry(key.timeframe)
            .or_default()
            .push(key.month);
    }
    for rungs in held.values_mut() {
        for months in rungs.values_mut() {
            months.sort_unstable();
            months.dedup();
        }
    }
    held
}

/// The census entry of one rung and month of `identity`, as
/// [`crate::calendar_of::CensusEntry`]: one probe.
fn entry_of(
    manifest: &census::Manifests,
    identity: SpotIdentity,
) -> impl Fn(store::path::Timeframe, store::path::YearMonth) -> Option<crate::calendar_of::CensusEntry>
+ '_ {
    move |rung, month| {
        let (exchange, segment, symbol) = identity;
        manifest
            .entry(
                &census::Series {
                    contract: None,
                    exchange,
                    segment,
                    symbol,
                    timeframe: rung,
                }
                .at(month),
            )
            .map(|entry| (entry.rows, entry.first_ts_micros, entry.last_ts_micros))
    }
}

/// What reading the evidence for one answer needs.
struct Evidence<'a> {
    site: &'a Site,
    feed: Vendor,
    manifest: &'a census::Manifests,
}

impl Evidence<'_> {
    /// Read `identity` at `rung`, or the refusal its unopened files earn.
    ///
    /// The reading is each held month's bars per day at `rung`
    /// ([`crate::calendar_of::month_days`]): two time lookups a day, and a
    /// month whose census entry has not moved since it was counted opens
    /// nothing. Every rung reads the same way, the daily one included: which
    /// days hold a daily bar is all a calendar needs of it, and deriving the
    /// series' whole calendar walked its minute files too, which cost about
    /// 0.3 s a series on every miss (DB-G85). A held file that does not open
    /// refuses the answer, as it always did (D-0695); a month that opened and
    /// would not be looked up is unknown, and named in `unreadable`.
    fn read(
        &self,
        identity: SpotIdentity,
        rungs: &HeldMonths,
        rung: store::path::Timeframe,
        unreadable: &mut Vec<String>,
    ) -> Result<crate::calendar_of::record::Reading, CalendarAnswer> {
        let (exchange, segment, symbol) = identity;
        let entry = entry_of(self.manifest, identity);
        let mut months = Vec::new();
        let mut unopened = Vec::new();
        let held = rungs.get(&rung).map_or(&[][..], Vec::as_slice);
        // HOW FAR A BUILD HAS GOT is what a "building" answer says (DB-N19).
        crate::calendar_of::build::owe(held.len());
        for month in held {
            let counted = crate::calendar_of::month_days(
                &self.site.calendars,
                &self.site.store_root,
                self.feed,
                exchange.as_str(),
                segment.as_str(),
                symbol.as_str(),
                rung,
                *month,
                entry(rung, *month),
            );
            crate::calendar_of::build::counted();
            match counted {
                Ok(days) => months.push((*month, Some(days))),
                Err(crate::calendar_of::MonthFault::Unopened(why)) => unopened.push(why),
                Err(crate::calendar_of::MonthFault::Unreadable(why)) => {
                    unreadable.push(format!("{}: {why}", symbol.as_str()));
                    months.push((*month, None));
                }
            }
        }
        if !unopened.is_empty() {
            return Err(unopened_calendar(self.feed, symbol.as_str(), &unopened));
        }
        Ok(crate::calendar_of::record::Reading::from_months(
            symbol.as_str(),
            &months,
        ))
    }
}

/// One built answer, and whether it may be kept under the feed's stamp.
enum Built {
    /// An answer, kept when `keep` says its key is one the census bounds.
    Answer {
        status: axum::http::StatusCode,
        body: String,
        keep: bool,
    },
    /// A refusal, never kept: the next request asks the disk again.
    Refused(CalendarAnswer),
}

impl Built {
    /// A `200` answer.
    const fn ok(body: String, keep: bool) -> Self {
        Self::Answer {
            status: axum::http::StatusCode::OK,
            body,
            keep,
        }
    }
}

/// The rung an answer is built at: the one asked for, or the feed's default,
/// which is the daily rung where the holders hold one (it reaches furthest
/// back) and their finest rung otherwise (GDFL's `1s`, D-2802).
fn chosen_rung(
    asked: Option<store::path::Timeframe>,
    holders: &[&HeldMonths],
) -> Option<store::path::Timeframe> {
    if asked.is_some() {
        return asked;
    }
    let held = || holders.iter().flat_map(|rungs| rungs.keys().copied());
    if held().any(|rung| rung == store::path::Timeframe::DAY_1) {
        return Some(store::path::Timeframe::DAY_1);
    }
    held().min_by_key(|rung| rung.secs())
}

/// Build the answer for `feed`, `rung` and `symbol` from the feed's census
/// and the bars it names. Runs once per feed generation; see
/// [`calendar_json_reading`].
#[expect(
    clippy::too_many_lines,
    reason = "one census walk shared by the exchange and the series branch, the identity \
              refusal, the rung choice, and each series read once whether it holds or attests"
)]
fn build(
    site: &Site,
    own: Option<&census::VendorCensus>,
    feed: Vendor,
    asked_rung: Option<store::path::Timeframe>,
    symbol: &str,
) -> Built {
    use crate::calendar_of::record::{self, Target};
    let generation = own.and_then(census::VendorCensus::generation);
    let unheld_target = |reason: String| {
        let target = if symbol.is_empty() {
            Target::Exchange
        } else {
            Target::Series {
                exchange: "",
                segment: "",
                symbol,
            }
        };
        record::withheld(feed.as_str(), asked_rung, target, generation, &reason)
    };
    let Some(manifest) = own.and_then(|row| match row.state {
        census::Census::Held { ref manifest } => Some(manifest),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    }) else {
        // AN ABSENT CENSUS IS THE ORDINARY STATE BEFORE A FIRST PULL, AND IT
        // HAS NO STAMP, SO NOTHING IS KEPT. An unreadable one was refused
        // before this. D-0124.
        return Built::ok(
            unheld_target(format!(
                "{} has no census: nothing has been ingested for this feed",
                feed.as_str()
            )),
            false,
        );
    };
    let held = held_series(manifest, symbol);
    // ONE IDENTITY PER ANSWER. A name held under two segments is two series,
    // and one calendar of both would ask for months of one under the other.
    let mut named = held
        .keys()
        .filter(|(_, _, name)| !symbol.is_empty() && name.as_str() == symbol);
    let target = named.next().copied();
    if let (Some(first), Some(second)) = (target, named.next()) {
        return Built::Refused((
            axum::http::StatusCode::CONFLICT,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )],
            calendar_conflict_json(feed, symbol, (first.0, first.1), (second.0, second.1)),
        ));
    }
    if !symbol.is_empty() && target.is_none() {
        // A NAME THIS FEED'S CENSUS DOES NOT HOLD IS ANSWERED AND NOT KEPT:
        // its key is the request's own text, and keeping it would let every
        // distinct unheld name grow the map for the life of the process.
        // D-0695.
        return Built::ok(
            unheld_target(format!(
                "{}'s census holds no spot series named {symbol}",
                feed.as_str()
            )),
            false,
        );
    }
    let references: Vec<(SpotIdentity, &HeldMonths)> = held
        .iter()
        .filter(|((exchange, segment, name), _)| {
            *exchange == brutex_core::instrument::Exchange::Nse
                && *segment == brutex_core::instrument::Segment::Index
                && REFERENCES.contains(&name.as_str())
        })
        .map(|(identity, rungs)| (*identity, rungs))
        .collect();
    let holders: Vec<(SpotIdentity, &HeldMonths)> = match target {
        Some(identity) => held
            .get(&identity)
            .map(|rungs| vec![(identity, rungs)])
            .unwrap_or_default(),
        None => references.clone(),
    };
    let target_json = target
        .as_ref()
        .map_or(Target::Exchange, |(exchange, segment, name)| {
            Target::Series {
                exchange: exchange.as_str(),
                segment: segment.as_str(),
                symbol: name.as_str(),
            }
        });
    let holding: Vec<&HeldMonths> = holders.iter().map(|(_, rungs)| *rungs).collect();
    let Some(rung) = chosen_rung(asked_rung, &holding) else {
        return Built::ok(
            record::withheld(
                feed.as_str(),
                asked_rung,
                target_json,
                generation,
                &format!(
                    "{} holds neither {} nor {} as a spot index: the exchange calendar is \
                     held by those two, and no other series attests a day",
                    feed.as_str(),
                    REFERENCES[0],
                    REFERENCES[1]
                ),
            ),
            true,
        );
    };
    if !holding.iter().any(|rungs| rungs.contains_key(&rung)) {
        let series: Vec<&str> = holders
            .iter()
            .map(|((_, _, name), _)| name.as_str())
            .collect();
        return Built::ok(
            record::withheld(
                feed.as_str(),
                Some(rung),
                target_json,
                generation,
                &format!(
                    "{} holds no {} bar for {}; its census holds {} for them",
                    feed.as_str(),
                    rung.as_str(),
                    series.join(" or "),
                    holding
                        .iter()
                        .flat_map(|rungs| rungs.keys().map(|r| r.as_str()))
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
            true,
        );
    }
    let evidence = Evidence {
        site,
        feed,
        manifest,
    };
    let mut unreadable = Vec::new();
    let mut read: std::collections::BTreeMap<SpotIdentity, record::Reading> =
        std::collections::BTreeMap::new();
    for (identity, rungs) in holders.iter().chain(references.iter()) {
        // EACH SERIES IS READ ONCE, whether it holds, attests or both, and only
        // at the rung asked: a reference that does not hold it attests nothing.
        if read.contains_key(identity) || !rungs.contains_key(&rung) {
            continue;
        }
        match evidence.read(*identity, rungs, rung, &mut unreadable) {
            Ok(reading) => {
                read.insert(*identity, reading);
            }
            Err(refused) => return Built::Refused(refused),
        }
    }
    let pick = |series: &[(SpotIdentity, &HeldMonths)]| -> Vec<record::Reading> {
        series
            .iter()
            .filter_map(|(identity, _)| read.get(identity).cloned())
            .collect()
    };
    let asked = record::Asked {
        feed: feed.as_str(),
        rung,
        rung_asked: asked_rung.is_some(),
        target: target_json,
        generation,
    };
    Built::ok(
        record::answer(&asked, &pick(&holders), &pick(&references), &unreadable),
        true,
    )
}

/// [`calendar_json`], with the census it derives from passed in.
///
/// Production passes [`census_now_stamped`] and nothing else. The parameter is
/// here so a test can install a newer manifest AFTER this route's census is
/// read and before its calendar is kept -- the interleaving that decides
/// whether the calendar is kept under its census's stamp or a later one -- on
/// every run, as
/// [`census_now_reading`](super::census_cache::census_now_reading)'s parameter
/// does for the census. D-0695.
///
/// # What it answers, since D-3336
///
/// The calendar of record over the span the feed holds, per rung: see
/// [`crate::calendar_of::record`]. A request is one census stamp and one map
/// probe when the answer for its feed, rung and symbol is kept under the
/// feed's current manifest stamp; otherwise the answer is built once
/// ([`build`]) and kept. A fresh census, never the boot snapshot (D-0318), on
/// the manifests' stamps (D-0686); an unreadable census is refused (D-0124);
/// a held file that does not open refuses the request and keeps nothing
/// (D-0695); a symbol the census does not hold is answered and not kept
/// (D-0695).
pub(super) fn calendar_json_reading(
    site: &Site,
    uri: &axum::http::Uri,
    census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
) -> CalendarAnswer {
    let json = "application/json; charset=utf-8";
    let Asked {
        feed,
        rung,
        symbol,
        key,
    } = match Asked::of(uri) {
        Ok(asked) => asked,
        Err(refused) => return refused,
    };
    // ONE STAMPED CENSUS, and a feed whose census cannot be read is refused
    // before anything is built. See `unreadable_calendar`.
    let (stamps, (fresh, _)) = census(site);
    // `census_now` yields one row per `Vendor::ALL` entry, so this sees at most
    // `FEED_COUNT` rows.
    let own = fresh.iter().find(|census| census.vendor == feed);
    if let Some(refused) = unreadable_calendar(own, feed) {
        return refused;
    }
    // THE KEY EVERY ANSWER AND CALENDAR IS KEPT UNDER: this feed's manifest as
    // it was stamped before `fresh` was read, and only when that stamp could
    // have read this feed's row. See `census_now_stamped`.
    let stamp = own.and_then(|row| stamps.modified(row));
    // AND THE STAMP A KEPT ANSWER IS CONFIRMED UNDER: the same stamps, this
    // feed's alone, so the next request is handed it without reading the
    // census while that feed's manifest stands still (DB-N19).
    let confirm = |kept| {
        site.calendars
            .builds
            .confirm(key.clone(), narrowed(&stamps, feed), kept);
    };
    if let Some(at) = stamp
        && let Some(kept) = site.calendars.answer(&key, at)
    {
        confirm(std::sync::Arc::clone(&kept));
        return (
            axum::http::StatusCode::from_u16(kept.status)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
            [(axum::http::header::CONTENT_TYPE, json)],
            kept.body.clone(),
        );
    }
    match build(site, own, feed, rung, &symbol) {
        Built::Answer { status, body, keep } => {
            if keep && let Some(at) = stamp {
                confirm(site.calendars.keep_answer(
                    key.clone(),
                    at,
                    crate::calendar_of::KeptAnswer {
                        status: status.as_u16(),
                        body: body.clone(),
                    },
                ));
            }
            (status, [(axum::http::header::CONTENT_TYPE, json)], body)
        }
        Built::Refused(refused) => refused,
    }
}

// ══ At start: the census and the answers the pages ask first (DB-N19) ═══════

/// Bound every `/calendar.json` request's wait to
/// [`crate::calendar_of::build::REQUEST_WAIT`], and build, in the background,
/// what the first clicks after a restart ask for: see [`warm`]. The served
/// process calls this once, after its listener is bound.
///
/// # Errors
///
/// The thread could not be started. The route still answers: a request then
/// leads a build of its own, and is answered `"building"` past the budget.
pub(super) fn warm_at_start(site: &Loaded) -> std::io::Result<()> {
    site.calendars
        .builds
        .bound_waits(crate::calendar_of::build::REQUEST_WAIT);
    let site = Loaded::clone(site);
    std::thread::Builder::new()
        .name("brutex-warm".to_owned())
        .spawn(move || {
            let _noted = warm(&site, &offered_feeds()).note();
        })
        .map(drop)
}

/// The feeds the operator is offered (D-3338): every feed whose descriptor
/// does not exclude it and that files under a store vendor.
pub(super) fn offered_feeds() -> Vec<Vendor> {
    pull::vendor::Feed::ALL
        .iter()
        .filter(|feed| matches!(feed.descriptor().offer, pull::vendor::Offer::Offered))
        .filter_map(|feed| feed.store_vendor())
        .collect()
}

/// The symbols every feed's first answers are built for: NIFTY, the exchange
/// and BANKNIFTY, in that order. `/db` opens on NIFTY (the operator's answer
/// 11), `/ingest` asks for the exchange, and the two indices are the series
/// the calendar of record is held by ([`REFERENCES`]).
const FIRST_SYMBOLS: [&str; 3] = ["NIFTY", "", "BANKNIFTY"];

/// The start-up builds not yet finished. Dropped -- at the end, or by a panic
/// part way -- it abandons every build still in it, so no request waits on a
/// build that will never answer; the next request leads a build of its own.
struct Queue<'a> {
    site: &'a Site,
    builds: std::collections::VecDeque<(
        crate::calendar_of::AnswerKey,
        std::sync::Arc<crate::calendar_of::build::Build>,
    )>,
}

impl Queue<'_> {
    /// Queue `key`, once.
    fn push(
        &mut self,
        key: crate::calendar_of::AnswerKey,
        warmed: &mut crate::calendar_of::build::Warmed,
    ) {
        if self.builds.iter().any(|(queued, _)| *queued == key) {
            return;
        }
        match self.site.calendars.builds.queue_at_start(&key) {
            Some(build) => self.builds.push_back((key, build)),
            None => warmed.unqueued = warmed.unqueued.saturating_add(1),
        }
    }
}

impl Drop for Queue<'_> {
    fn drop(&mut self) {
        for (key, build) in self.builds.drain(..) {
            self.site.calendars.builds.abandon(
                &key,
                &build,
                crate::calendar_of::KeptAnswer {
                    status: axum::http::StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                    body: serde_json::json!({
                        "status": "refused",
                        "feed": key.0.as_str(),
                        "refusal": "the start-up build stopped before this answer was built; \
                                    ask again, and the request leads a build of its own",
                    })
                    .to_string(),
                },
            );
        }
    }
}

/// The rungs `feeds`' censuses hold NIFTY or BANKNIFTY at, finest first.
fn reference_rungs(
    fresh: &[census::VendorCensus],
    feeds: &[Vendor],
) -> Vec<(Vendor, store::path::Timeframe)> {
    let mut rungs = Vec::new();
    for feed in feeds {
        let Some(manifest) = fresh
            .iter()
            .find(|row| row.vendor == *feed)
            .and_then(|row| match row.state {
                census::Census::Held { ref manifest } => Some(manifest),
                census::Census::Absent | census::Census::Unreadable { .. } => None,
            })
        else {
            continue;
        };
        let mut held: Vec<store::path::Timeframe> = held_series(manifest, "")
            .values()
            .flat_map(|by_rung| by_rung.keys().copied())
            .collect();
        held.sort_unstable_by_key(|rung| rung.secs());
        held.dedup();
        rungs.extend(held.into_iter().map(|rung| (*feed, rung)));
    }
    rungs
}

/// The question a start-up build asks, as a request would ask it.
fn question(key: &crate::calendar_of::AnswerKey) -> Result<axum::http::Uri, CalendarAnswer> {
    let (feed, rung, symbol) = key;
    let mut text = format!("/calendar.json?feed={}", feed.as_str());
    if let Some(rung) = rung {
        text.push_str("&rung=");
        text.push_str(rung.as_str());
    }
    if !symbol.is_empty() {
        text.push_str("&symbol=");
        text.push_str(symbol);
    }
    text.parse().map_err(|why| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )],
            serde_json::json!({
                "status": "refused",
                "refusal": format!("the start-up build could not spell {text:?} as a request: {why}"),
            })
            .to_string(),
        )
    })
}

/// Read the census once and build the answers the first clicks after a
/// restart ask for, one after another, while the pull journal is walked
/// beside them (`dayflags::warm_pull_journal`). DB-N19.
///
/// The first answers of every feed -- NIFTY, the exchange and BANKNIFTY at
/// the feed's default rung -- are queued before the census is read, so a
/// request for one joins its build rather than starting a second; then every
/// rung the feed holds NIFTY or BANKNIFTY at, finest first. Each is built
/// exactly as a request builds it ([`calendar_json_reading`]), kept and
/// confirmed the same way, and finished for whoever waits on it. While the
/// census is read, a request for any other key is answered `"building"`
/// rather than reading it beside this.
pub(super) fn warm(site: &Site, feeds: &[Vendor]) -> crate::calendar_of::build::Warmed {
    let started = std::time::Instant::now();
    let mut warmed = crate::calendar_of::build::Warmed {
        feeds: feeds.to_vec(),
        census_ms: 0,
        built: 0,
        refused: 0,
        unqueued: 0,
        building_ms: 0,
        journal: Ok((0, 0)),
        total_ms: 0,
    };
    let mut queue = Queue {
        site,
        builds: std::collections::VecDeque::new(),
    };
    for feed in feeds {
        for symbol in FIRST_SYMBOLS {
            queue.push((*feed, None, symbol.to_owned()), &mut warmed);
        }
    }
    let root = site.store_root.clone();
    let journal = std::thread::Builder::new()
        .name("brutex-warm-journal".to_owned())
        .spawn(move || super::dayflags::warm_pull_journal(&root));
    let census_began = std::time::Instant::now();
    let fresh = {
        let _reading = site.calendars.builds.reading_census_at_start();
        census_now_stamped(site).1.0
    };
    warmed.census_ms = millis(census_began.elapsed());
    for (feed, rung) in reference_rungs(&fresh, feeds) {
        for symbol in FIRST_SYMBOLS {
            queue.push((feed, Some(rung), symbol.to_owned()), &mut warmed);
        }
    }
    let building = std::time::Instant::now();
    // THE BUILD STAYS IN THE QUEUE UNTIL IT IS FINISHED, so one that unwinds
    // part way is abandoned by the queue's drop rather than left running.
    while let Some((key, build)) = queue.builds.front().cloned() {
        let answer = match question(&key) {
            Ok(uri) => {
                let _entered = build.enter(Some(feed_stamps(&site.store_root, key.0)));
                calendar_json_reading(site, &uri, census_now_stamped)
            }
            Err(refused) => refused,
        };
        warmed.built = warmed.built.saturating_add(1);
        if answer.0 != axum::http::StatusCode::OK {
            warmed.refused = warmed.refused.saturating_add(1);
        }
        site.calendars.builds.finish(&key, &build, kept_of(answer));
        queue.builds.pop_front();
    }
    warmed.building_ms = millis(building.elapsed());
    warmed.journal = match journal {
        Ok(walking) => match walking.join() {
            Ok(Ok(walked)) => Ok((walked.read, walked.micros / 1_000)),
            Ok(Err(why)) => Err(why),
            Err(_) => Err("the pull journal's walk at start unwound".to_owned()),
        },
        Err(why) => Err(format!("the pull journal's walk could not start: {why}")),
    };
    warmed.total_ms = millis(started.elapsed());
    warmed
}
