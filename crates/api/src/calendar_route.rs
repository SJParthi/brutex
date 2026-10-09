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
    // OFF THE ASYNC WORKERS, AND ADMITTED. This ran inline: a census read on a
    // moved stamp and, on a miss, a derivation per spot series (0.28 s each,
    // documented) blocked a Tokio worker, and a flood or a post-pull herd
    // blocked them all. It now runs on the blocking pool behind its own
    // bounded admission. Saturation answers 429 with a reason rather than
    // queueing without bound. W1-api2-11, D-1443. Since D-3336 a miss reads
    // only the months whose census entry moved (`calendar_of::month_days`),
    // so two concurrent misses repeat at most that much work.
    match crate::detail::run_calendar(move || {
        calendar_json_reading(&site, &uri, census_now_stamped)
    })
    .await
    {
        Ok(answer) => answer,
        Err(why) => calendar_admission_refused(&why),
    }
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
    manifest: &pull::manifest::Manifest,
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
    manifest: &pull::manifest::Manifest,
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
    manifest: &'a pull::manifest::Manifest,
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
        for month in rungs.get(&rung).map_or(&[][..], Vec::as_slice) {
            match crate::calendar_of::month_days(
                &self.site.calendars,
                &self.site.store_root,
                self.feed,
                exchange.as_str(),
                segment.as_str(),
                symbol.as_str(),
                rung,
                *month,
                entry(rung, *month),
            ) {
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
        census::Census::Held { ref manifest } => Some(manifest.as_ref()),
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
    let query = uri.query().unwrap_or("");
    let asked = param(query, "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, json)],
            no_such_feed_json(&asked),
        );
    };
    let rung = match rung_param(query) {
        Ok(rung) => rung,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                [(axum::http::header::CONTENT_TYPE, json)],
                serde_json::json!({ "error": why }).to_string(),
            );
        }
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
    let symbol = stored_case(param(query, "symbol"));
    let key: crate::calendar_of::AnswerKey = (feed, rung, symbol.clone());
    if let Some(at) = stamp
        && let Some(kept) = site.calendars.answer(&key, at)
    {
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
                site.calendars.keep_answer(
                    key,
                    at,
                    crate::calendar_of::KeptAnswer {
                        status: status.as_u16(),
                        body: body.clone(),
                    },
                );
            }
            (status, [(axum::http::header::CONTENT_TYPE, json)], body)
        }
        Built::Refused(refused) => refused,
    }
}
