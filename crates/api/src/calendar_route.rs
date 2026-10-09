//! `/calendar.json` -- the trading calendar derived from a feed's stored
//! spot series, and the refusals a calendar-deriving route answers with.
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
    CensusNow, CensusStamps, Loaded, Site, census_holds, census_now_stamped, no_such_feed_json,
    param, stored_case,
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
    // bounded admission, and concurrent misses on one series share one
    // derivation (`calendar_of::Cache`). Saturation answers 429 with a reason
    // rather than queueing without bound. W1-api2-11, D-1443.
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

/// [`calendar_json`], with the census it derives from passed in.
///
/// Production passes [`census_now_stamped`] and nothing else. The parameter is
/// here so a test can install a newer manifest AFTER this route's census is
/// read and before its calendar is kept -- the interleaving that decides
/// whether the calendar is kept under its census's stamp or a later one -- on
/// every run, as
/// [`census_now_reading`](super::census_cache::census_now_reading)'s parameter
/// does for the census. D-0695.
#[expect(
    clippy::too_many_lines,
    reason = "one stamped census shared by both branches, refused once before either when \
              it cannot be read; splitting the branches apart would re-take it per branch"
)]
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
    // ONE STAMPED CENSUS FOR EITHER BRANCH, and a feed whose census cannot be
    // read is refused before either derives anything. See `unreadable_calendar`.
    // Its stamps key every calendar either branch keeps: see
    // `census_now_stamped`.
    let (stamps, (fresh, _)) = census(site);
    // THE ASKED FEED'S OWN ROW, FOUND ONCE: the refusal just below reads its
    // state, and each branch asks it through `census_holds` whether a bar file
    // a derivation could not open is one it holds. `census_now` yields one row
    // per `Vendor::ALL` entry, so this sees at most `FEED_COUNT` rows.
    let own = fresh.iter().find(|census| census.vendor == feed);
    if let Some(refused) = unreadable_calendar(own, feed) {
        return refused;
    }
    // THE KEY EVERY CALENDAR BELOW IS KEPT UNDER: this feed's manifest as it
    // was stamped before `fresh` was read, never a `stat` taken after it, and
    // only when that stamp could have read this feed's row. See
    // `census_now_stamped` and `CensusStamps::modified`.
    let stamp = own.and_then(|row| stamps.modified(row));
    // NO SYMBOL MEANS THE EXCHANGE'S OWN CALENDAR, AGREED RATHER THAN BORROWED.
    //
    // This used to answer 400, on the reasoning that a derived calendar is one
    // instrument's reading and there is no default that is not a guess. The
    // reasoning was right and the conclusion was wrong: the caller that needs
    // this — `/ingest`, computing how many bars a month owed — is asking an
    // EXCHANGE question, and refusing it left the browser holding four
    // hardcoded tables of the same facts, which is the duplication D-0274
    // recorded (P-03 itself was closed by D-2673).
    //
    // The honest answer is not to pick an instrument. It is to derive every
    // instrument this feed holds and agree them, and to ship the agreement's
    // own evidence beside it — who it was derived from, and every day they read
    // differently. See `calendar_of::agree` for why that is a union.
    let symbol = stored_case(param(query, "symbol"));
    if symbol.is_empty() {
        // A FRESH CENSUS, NOT `site.entries`.
        //
        // `entries` is filled once in `Site::load` and never again — D-0039 —
        // so a server started before its first pull holds an EMPTY list for
        // life. This route then names no instruments, walks no months, and
        // answers `{"sessions":0,"days":[]}`.
        //
        // That is not a quiet failure; it is a loud wrong answer three hops
        // downstream. `web/`'s `isSession` uses the measured calendar where it
        // has one and falls back to a bare weekday rule where it does not — so
        // an empty calendar makes the page count all 92 public holidays in this
        // window as sessions, expect 1,758 daily bars instead of the 1,671 that
        // exist, and print **SHORT against 204 of 210 complete instruments**.
        // The operator reads a broken pull; the pull is fine.
        //
        // `crates/pull/src/calendar.rs` was written for exactly this: *"stored
        // series as SHORT when every one of them was complete. An operator
        // cannot tell a real hole from a public holiday, so every alarm is
        // noise and a true gap hides among them."* It could not do that job
        // through a list captured before the store had anything in it.
        //
        // `broker_run`'s ladder gate hit the identical defect and its comment
        // says so — *"The documented workaround was to restart the server,
        // which is a workaround for a bug."* It was fixed there by taking a
        // fresh census; this is the same fix, on the route that reports rather
        // than the one that gates. D-0318.
        //
        // FRESH ON THE MANIFESTS' STAMPS, which keeps D-0318 and drops the
        // per-request manifest read. This was a direct `read_all` -- every
        // vendor's whole manifest, read on every request -- beside a cache
        // that already answered the same question from one `stat` per vendor.
        // A rewritten manifest moves its stamp and is re-read. D-0686. Taken
        // once, above the branch, as `fresh`.
        let mut readings: Vec<(String, pull::calendar::Calendar)> = Vec::new();
        // THE SAME FRESH READING, WALKED ONCE. Filtering `site.entries` here
        // would have re-introduced the staleness two lines after fixing it —
        // the names would come from disk and the months from the boot snapshot,
        // so every name would resolve to zero months and the calendar would
        // still be empty. Collected before the loop because `read_all` walks
        // the manifests and doing it per symbol would be one walk per name.
        // GROUPED ONCE, PROBED PER NAME. Filtering the whole list per symbol is
        // 205 names × 15,857 rows on this operator's store — 3.2 million string
        // compares to answer a question a map answers in one probe each. One
        // pass to build, O(1) per name after it: `CLAUDE.md` §3 rule 4.
        //
        // KEYED ON THE WHOLE IDENTITY, NOT THE BARE NAME, and that is the fix.
        // This map was keyed on `series.symbol` alone, and the two arguments
        // below were the literals `"NSE"` and `"INDEX"` — for every symbol,
        // whatever the census actually said. `Series` carries `exchange`,
        // `segment` AND `contract`, all three populated from the same
        // `EntryKey` `pull::ingest` wrote the file under; dropping them here
        // left nothing to pass, so a constant was substituted.
        //
        // MEASURED: `ADANIENT` is stored at `NSE/CASH/ADANIENT/`, exactly where
        // the master's `instrument_type` column says an `EQ` row belongs. This
        // route asked `NSE/INDEX/ADANIENT/` **366 times** — 61 held months × 2
        // rungs × 3 cache-missing derivations — and every probe missed. 1,240
        // daily bars sat on disk reading as "no data", which is the shape
        // `CLAUDE.md` §4 bans outright: a fallback that hides a failure.
        //
        // The mirror image was already fixed on the WRITE path, and the comment
        // left there predicted this exact symptom: *"a later reader asking for
        // `NSE/CASH/360ONE` finds nothing while the data sits one directory
        // over"*. Same defect, opposite side, and the write side was the half
        // that got repaired.
        // THIS FEED'S CENSUS, NOT EVERY FEED'S.
        //
        // `held_entries` flattens `Vec<VendorCensus>` and `Series` carries NO
        // vendor — the type's own doc says so — so months from all three feeds
        // were merged and then probed against the ONE feed being derived. The
        // floors differ by years: Zerodha reaches 2015, Groww 2020, Dhan a
        // rolling five. So Dhan's derivation opened months back to 2015 and
        // every one of them refused, loudly, into the rolling log.
        //
        // MEASURED during a live repull: the newest refusal named
        // `dhan/NSE/INDEX/NIFTY/1min/2019-12.bin`, a month Dhan's own floor puts
        // outside its history — and which no feed had asked it for.
        //
        // `VendorCensus::vendor` is right there; filtering by it is one
        // comparison per census, of which there are five.
        //
        // BORROWED FROM THE SHARED CENSUS, NOT CLONED OUT OF IT. `VendorCensus`
        // boxes a whole `Manifest`, so the `.cloned()` that stood here
        // deep-copied this feed's every entry on every request -- the copy
        // D-0171 removed from `/store` -- and would have undone what sharing
        // the cached census by `Arc` saves. `read_all` answers one census per
        // `Vendor::ALL`, so at most one matches and this is the same list
        // `held_entries` built from the clone. D-0686.
        let mine: Vec<(census::Series, store::path::YearMonth)> = fresh
            .iter()
            .filter(|census| census.vendor == feed)
            .flat_map(|census| census::held_entries(std::slice::from_ref(census)))
            .collect();
        let by_series = spot_months_by_identity(&mine);
        // SORTED SO THE ANSWER IS REPRODUCIBLE. A `HashMap`'s iteration order
        // varies per process, and `agree` ships the names it derived from —
        // an unsorted walk would reorder `from` between two identical requests.
        let mut keys: Vec<_> = by_series.keys().copied().collect();
        keys.sort_unstable();
        for key in keys {
            let (exchange, segment, symbol) = key;
            let months: Vec<store::path::YearMonth> =
                by_series.get(&key).cloned().unwrap_or_default();
            let derived = crate::calendar_of::cached(
                &site.calendars,
                &site.store_root,
                feed,
                exchange.as_str(),
                segment.as_str(),
                symbol.as_str(),
                stamp,
                &months,
                census_holds(own.map(|census| (census, key))),
            );
            // ONE INSTRUMENT'S HELD FILE THAT DID NOT OPEN REFUSES THE AGREEMENT.
            // Left out, it votes for nothing, which the union reads as no
            // session wherever it was the one witness, and that agreement would
            // be answered as the exchange's. See `unopened_calendar`.
            if !derived.unopened.is_empty() {
                return unopened_calendar(feed, symbol.as_str(), &derived.unopened);
            }
            let calendar = derived.calendar;
            // A SYMBOL THIS FEED HOLDS NOTHING FOR IS NOT A VOTE FOR ANYTHING.
            // `entries` spans every vendor, so asking Zerodha's store for an
            // instrument only Dhan carries reads no files and returns an empty
            // calendar; counting that as agreement would let a feed's absence
            // close the exchange.
            if calendar.sessions() > 0 {
                // Cloned off the `Arc` here rather than inside the cache's
                // lock -- see `calendar_of::Cache`.
                readings.push((symbol.as_str().to_owned(), (*calendar).clone()));
            }
        }
        let (exchange, clashes) = crate::calendar_of::agree(&readings);
        let from: Vec<String> = readings.into_iter().map(|(name, _)| name).collect();
        return (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, json)],
            crate::calendar_of::exchange_json(&exchange, &from, &clashes),
        );
    }

    // EVERY MONTH THE CENSUS NAMES FOR THIS FEED. Asking the census rather than
    // walking `bars/` is layer 13 of `docs/07-o1-architecture.md`: the counter
    // answers what a directory walk would take ~248,000 `stat` calls to learn.
    // `Series` carries no vendor — the censuses are already per feed — so the
    // symbol is the whole filter here and the feed is carried by `store_root`
    // plus the path the derivation builds.
    // THE SAME FRESH READ ON THIS BRANCH TOO, and it is a separate one: fixing
    // only the branch above would leave `/calendar.json?symbol=NIFTY` answering
    // from the boot snapshot while `/calendar.json` answered from disk. Two
    // routes on one handler disagreeing about what the store holds is worse
    // than both being stale, because only one of them is ever checked.
    // THE ASKED SYMBOL'S OWN IDENTITY, READ FROM THE CENSUS RATHER THAN ASSUMED.
    // This branch carried the same two literals the exchange branch did, and
    // the same consequence: `/calendar.json?symbol=ADANIENT` probed
    // `NSE/INDEX/ADANIENT/` while the bars sat in `NSE/CASH/ADANIENT/`.
    let mut held: Option<(
        brutex_core::instrument::Exchange,
        brutex_core::instrument::Segment,
    )> = None;
    // The stored name that goes with `held`, kept for `census_holds`.
    let mut named: Option<brutex_core::symbol::Symbol> = None;
    let mut months: Vec<store::path::YearMonth> = Vec::new();
    // THIS FEED'S CENSUS ONLY — the same filter the exchange branch takes, and
    // for the same measured reason: `Series` carries no vendor, so without it
    // Zerodha's months (back to 2015) are probed against Dhan's store (a
    // rolling five years) and every one refuses into the log.
    //
    // THE SAME STAMPED CENSUS the exchange branch reads, so the two branches
    // still agree about what the store holds and neither re-reads an unchanged
    // manifest per request. D-0686. It is `fresh`, taken above the branch.
    for (series, month) in fresh
        .iter()
        .filter(|census| census.vendor == feed)
        .flat_map(|c| census::held_entries(std::slice::from_ref(c)))
    {
        if series.contract.is_some() || series.symbol.as_str() != symbol {
            continue;
        }
        let identity = (series.exchange, series.segment);
        // ONE IDENTITY PER ANSWER. A name held under two segments is two
        // different series, and merging their months would build one calendar
        // out of two paths — asking for months of the second under the first,
        // which is the very confusion this edit removes.
        match held {
            None => {
                held = Some(identity);
                named = Some(series.symbol);
                months.push(month);
            }
            Some(seen) if seen == identity => months.push(month),
            Some(seen) => {
                return (
                    axum::http::StatusCode::CONFLICT,
                    [(axum::http::header::CONTENT_TYPE, json)],
                    calendar_conflict_json(feed, &symbol, seen, identity),
                );
            }
        }
    }

    // A NAME THE CENSUS DOES NOT HOLD HAS NO PATH TO PROBE, and this is the
    // ONLY arm where the two words are not the symbol's own. `derive` over zero
    // months opens no file, so nothing here reaches a path; naming the case
    // explicitly is what stops a literal creeping back in, because the code
    // this replaces passed `"NSE"`/`"INDEX"` on every call, held or not.
    // ONE ENTRY PER MONTH, NOT ONE PER RUNG — the same dedup
    // `spot_months_by_identity` does, for the same reason: a month held at nine
    // rungs arrives nine times and would be probed nine times.
    months.sort_unstable();
    months.dedup();

    let (exchange, segment) = held.unwrap_or((
        brutex_core::instrument::Exchange::Nse,
        brutex_core::instrument::Segment::Index,
    ));

    let series = held
        .zip(named)
        .map(|((exchange, segment), symbol)| (exchange, segment, symbol));
    // A NAME THIS FEED'S CENSUS DOES NOT HOLD IS DERIVED FOR ITS OWN REQUEST
    // AND NOT KEPT. Its key is built from the request's own text, and nothing
    // evicts from `site.calendars`, so keeping it let every distinct unheld
    // name grow the map for the life of the process: a review sent 2,000 and
    // the map went from 9 entries to 2,009. With no stamp, `cached` neither
    // looks it up nor keeps it, and a derivation over no months opens no file.
    // The map is then bounded by the identities the feeds' censuses have held.
    // D-0695.
    let derived = crate::calendar_of::cached(
        &site.calendars,
        &site.store_root,
        feed,
        exchange.as_str(),
        segment.as_str(),
        &symbol,
        held.and(stamp),
        &months,
        census_holds(own.zip(series)),
    );
    // A HELD MONTH THAT DID NOT OPEN IS NOT A MONTH WITHOUT SESSIONS. See
    // `unopened_calendar`.
    if !derived.unopened.is_empty() {
        return unopened_calendar(feed, &symbol, &derived.unopened);
    }
    let calendar = derived.calendar;
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, json)],
        crate::calendar_of::json(&calendar),
    )
}
