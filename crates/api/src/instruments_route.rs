//! `/instruments.json` and `/universes.json` -- the selected feed's tracked
//! instruments with their stored bar counts, and what each spot target
//! resolves to for that feed.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The edits are visibility qualifiers
//! (`pub(super)`) on items another module now has to reach, and one probe:
//! `bars_by_symbol` asks [`census::VendorCensus::rows_of`] where it asked
//! `rows_for`, keeping only the `Held` arm -- exactly the `Some` arm it kept
//! before, so the sums are the same. That gives the WP-01 interface a
//! production caller. Intra-doc links to `server.rs` items that are no longer
//! in scope here carry their path. No route answers differently because of the
//! move.

use super::{
    Loaded, MasterState, Site, census_is_unreadable, no_such_feed_json, note_header, param,
    universe_label, universes_json, vendor_census,
};
use crate::{ingest, render};
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The browser's copy of the bounded universe, for the type-ahead.
///
/// # Why the whole set, and why once
///
/// The engine surface is bounded: ~750 NIFTY Total Market equities plus ~35 NSE
/// indices. That is small enough to hand the browser the entire searchable set
/// in one response, which is what makes a keystroke a Map probe instead of a
/// request. A request per character is ~800 requests to type one symbol and
/// makes keystroke latency a function of the network.
///
/// # Two silent answers this route used to give, and what now separates them
///
/// * **The counter would not load.** The page drew the same em dash a
///   genuinely un-pulled instrument shows. It now answers `503` with the
///   census headers [`super::store_route::census_headers_with`] writes.
/// * **The selected feed's master would not decode.** The body was `[]`,
///   indistinguishable from a feed that lists nothing. [`MASTER_STATE_HEADER`]
///   says which. D-0124.
///
/// # What D-3327 changed (DB-041, DB-042, DB-066, DB-G80)
///
/// * **Rows per rung.** `bars` counted one-minute spot bars only, so RELIANCE
///   with 2,168 day bars and NIFTYFINSERVICE with 2,910 read `0` and sorted
///   last. Each row now carries `rows_by_rung`, and `bars` is the row count of
///   the finest rung the store holds for it, named by `bars_rung`.
/// * **A feed with no master is listed from its census.** GDFL publishes no
///   instrument master, so this answered `[]` and every universe tier was
///   disabled. Its rows now come from the census: every stored spot series,
///   and every option underlying with no spot series (`spot: false`), each
///   with `master: null`, its membership from `core::universe`'s own lists,
///   and never a claim to be "in this master". A mastered feed's rows say
///   `master: "<feed>"`.
/// * **Off the async workers, and once per census read.** The body is built
///   on the store-read pool from the feed's facet index (one node per
///   instrument, `facets.rs`), and kept per (feed census read, universe
///   parse): an unchanged census answers the kept body. It walked every
///   census entry on every request, and on a miss rebuilt the census inline
///   on a Tokio worker for 360 to 400 ms.
pub(super) async fn instruments_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    // REFUSED BY NAME, NEVER SUBSTITUTED. `ingest::parse_vendor` answers the
    // EMPTY string with Dhan itself, so this caught exactly one input: a feed
    // somebody NAMED and this build does not read.
    let asked_feed = param(uri.query().unwrap_or(""), "feed");
    let Some(feed) = ingest::parse_vendor(&asked_feed) else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            json_headers(),
            no_such_feed_json(&asked_feed),
        );
    };
    match crate::detail::run_store_read(move || instruments_answer(&site, feed)).await {
        Ok(answer) => answer,
        Err(why) => {
            let saturated = matches!(why, crate::detail::RunError::Saturated);
            let mut headers = json_headers();
            if saturated {
                headers.insert(
                    axum::http::header::RETRY_AFTER,
                    axum::http::HeaderValue::from_static("1"),
                );
            }
            (
                if saturated {
                    axum::http::StatusCode::TOO_MANY_REQUESTS
                } else {
                    axum::http::StatusCode::SERVICE_UNAVAILABLE
                },
                headers,
                format!(
                    r#"{{"error":{}}}"#,
                    render::json_string(&format!("instrument read unavailable: {why:?}"))
                ),
            )
        }
    }
}

fn json_headers() -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers
}

/// The whole `/instruments.json` answer for one feed, off the async workers.
pub(super) fn instruments_answer(
    site: &Site,
    feed: Vendor,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let (_stamps, now) = vendor_census(site, feed);
    let census = now.census();
    // THE GUARD TAKEN ONCE, AND HELD ACROSS THE BUILD. The rows borrow from
    // the parsed universe, and a second `read()` on this lock while a
    // `Site::reparse` writer waits is a deadlock, not a slow path.
    let universe = site.universe();
    let body = site.census_wire.kept(
        now.slot(),
        feed,
        "instruments.json",
        universe.generation,
        || Ok(instruments_body(site, feed, now.slot(), &universe.read)),
    );
    let (master, master_note) = universe.read.master(feed);
    let behind = super::store_route::behind(site, feed);
    let mut headers = super::store_route::census_headers_with(census, Some(&behind));
    headers.insert(
        axum::http::HeaderName::from_static(UNIVERSE_STATUS_HEADER),
        note_header(universe.read.status()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_STATE_HEADER),
        axum::http::HeaderValue::from_static(master.name()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_NOTE_HEADER),
        note_header(&master_note),
    );
    // 503 FOR THE STATES WHERE EVERY NUMBER BELOW IS ABSENT RATHER THAN
    // MEASURED: a counter that will not load, or a master that should be there
    // and will not decode. A feed that publishes no master is not one of them:
    // its rows are the census's (D-3327).
    let code = if census_is_unreadable(census)
        || (master == MasterState::Unavailable && feed.publishes_master())
    {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    match body {
        Ok(body) => (code, headers, body.as_ref().clone()),
        Err(why) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        ),
    }
}

/// What the census holds for one spot listing: rows per rung, and whether
/// the feed holds options on it.
#[derive(Debug, Default, Clone)]
struct Held {
    rows: BTreeMap<store::path::Timeframe, u64>,
    options: bool,
}

impl Held {
    /// The finest rung held and its rows: what `bars` reports.
    fn bars(&self) -> (u64, Option<store::path::Timeframe>) {
        self.rows
            .iter()
            .find(|(_, rows)| **rows > 0)
            .map_or((0, None), |(rung, rows)| (*rows, Some(*rung)))
    }
}

/// The census's spot series and option underlyings for one feed, read off its
/// facet index: one node per instrument, never a walk of the census.
fn held_by_listing(
    site: &Site,
    feed: Vendor,
    slot: &std::sync::Arc<super::census_cache::Slot>,
) -> BTreeMap<(Exchange, Segment, Symbol), Held> {
    let mut held: BTreeMap<(Exchange, Segment, Symbol), Held> = BTreeMap::new();
    for segment in [Segment::Index, Segment::Cash] {
        super::facets::index(site, feed, segment, slot, |index| {
            for ((exchange, symbol), under) in index.underlyings() {
                held.entry((*exchange, segment, *symbol)).or_default().rows =
                    under.agg.rows.clone();
            }
        });
    }
    let options: Vec<(Exchange, Symbol)> =
        super::facets::index(site, feed, Segment::Fno, slot, |index| {
            index.underlyings().map(|(id, _)| *id).collect()
        });
    for (exchange, symbol) in options {
        let spot_segment = [Segment::Index, Segment::Cash]
            .into_iter()
            .find(|segment| held.contains_key(&(exchange, *segment, symbol)));
        // AN UNDERLYING WITH OPTIONS AND NO SPOT SERIES is listed under FNO,
        // so it is findable and says plainly that its spot is not stored.
        held.entry((exchange, spot_segment.unwrap_or(Segment::Fno), symbol))
            .or_default()
            .options = true;
    }
    held
}

/// The `/instruments.json` body for one feed: its master's tracked rows, or,
/// for a feed that publishes no master, the census's own listings.
fn instruments_body(
    site: &Site,
    feed: Vendor,
    slot: &std::sync::Arc<super::census_cache::Slot>,
    read: &super::Read,
) -> String {
    let held = held_by_listing(site, feed, slot);
    let mut rows: Vec<Row> = if feed.publishes_master() {
        read.merged
            .by_key
            .iter()
            .filter(|(_, entry)| {
                crate::catalog::tracked(entry.universe)
                    && entry.ids.get(feed as usize).copied().flatten().is_some()
            })
            .map(|(key, entry)| Row {
                key: *key,
                universe: entry.universe,
                held: held
                    .get(&(key.exchange, key.segment, key.underlying))
                    .cloned()
                    .unwrap_or_default(),
                master: Some(feed),
                spot: true,
            })
            .collect()
    } else {
        held.iter()
            .map(|(&(exchange, segment, symbol), held)| {
                let is_spot = segment != Segment::Fno;
                let key = InstrumentKey {
                    exchange,
                    segment,
                    underlying: symbol,
                    kind: if segment == Segment::Index {
                        Kind::Index
                    } else {
                        Kind::Equity
                    },
                };
                Row {
                    key,
                    // MEMBERSHIP FROM THE SOURCED LISTS, never from the census:
                    // being stored is not being a constituent (D-0089).
                    universe: brutex_core::universe::of_instrument(&key),
                    held: held.clone(),
                    master: None,
                    spot: is_spot,
                }
            })
            .collect()
    };
    // HELD FIRST, INDICES BEFORE EQUITIES, THEN BY NAME: the rows that can be
    // charted, the engine's own surface, then alphabetical.
    rows.sort_unstable_by_key(|row| {
        (
            row.held.bars().0 == 0,
            row.key.kind != Kind::Index,
            row.key.underlying,
            row.key.segment,
        )
    });
    let mut out = String::from("[");
    for (n, row) in rows.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        row.write(&mut out);
    }
    out.push(']');
    out
}

/// One `/instruments.json` row.
struct Row {
    key: InstrumentKey,
    universe: brutex_core::universe::Universe,
    held: Held,
    /// The feed whose master lists it, or `None` for a census-derived row.
    master: Option<Vendor>,
    /// Whether this is a spot listing; `false` for an option underlying the
    /// feed holds no spot series of.
    spot: bool,
}

impl Row {
    fn write(&self, out: &mut String) {
        let (bars, rung) = self.held.bars();
        let canonical = self.key.to_string();
        let _ = write!(
            out,
            // EXCHANGE AND SEGMENT TRAVEL WITH THE ROW, because the store is
            // keyed on them. `universe` is the frozen legacy label and
            // `universes` the whole set (D-0089): both load-bearing.
            r#"{{"symbol":{},"key":{},"kind":{},"exchange":{},"segment":{},"universe":{},"universes":{},"bars":{bars},"bars_rung":{},"rows_by_rung":{{"#,
            render::json_string(self.key.underlying.as_str()),
            render::json_string(&canonical),
            if self.spot {
                render::json_string(&format!("{:?}", self.key.kind))
            } else {
                "null".to_owned()
            },
            render::json_string(self.key.exchange.as_str()),
            render::json_string(self.key.segment.as_str()),
            render::json_string(&universe_label(self.universe)),
            universes_json(self.universe),
            rung.map_or_else(
                || "null".to_owned(),
                |rung| render::json_string(rung.as_str())
            ),
        );
        for (n, (rung, rows)) in self.held.rows.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}:{rows}", render::json_string(rung.as_str()));
        }
        let _ = write!(
            out,
            r#"}},"spot":{},"options":{},"master":{},"href":{}}}"#,
            self.spot,
            self.held.options,
            self.master.map_or_else(
                || "null".to_owned(),
                |feed| render::json_string(feed.as_str())
            ),
            render::json_string(&format!(
                "/instruments?q={}",
                render::query_value(&canonical)
            )),
        );
    }
}

/// What each spot target resolves to FOR ONE FEED — the counts, and every name
/// that did not resolve.
///
/// # The route the `/ingest` universe menu was missing
///
/// That menu drew four NIFTY tiers as `no target` and could not do otherwise:
/// nothing on the wire said what a tier resolves to for the selected feed, so a
/// page could either disable the row or synthesise a request the server cannot
/// honour. `/instruments.json` carries the membership — the `universes` array
/// has named `n500`, `n200`, `n100` and `n50` since D-0089 — but membership is
/// not reachability. A NIFTY 50 constituent that Groww's master has no row for
/// is in the tier and cannot be fetched from that feed, and folding
/// `/instruments.json` rows counts it or does not depending on which of two
/// filters a browser happens to apply.
///
/// This answers the question directly, once per feed: the slug to POST as
/// `target=`, the number that belongs on the control, and the buckets behind
/// the shortfall. D-0120.
///
/// # Cost
///
/// One [`coverage::Coverage::of`](crate::coverage::Coverage::of) per target —
/// seven array indices — plus the string. Nothing walks the universe:
/// [`Read::coverage`](super::Read::coverage) was built at load
/// (D-0039/D-0042), and the only thing that grows with the answer is the
/// `unresolved` list, which is the whole reason the operator asked.
///
/// A feed this build cannot read is REFUSED BY NAME rather than answered as
/// Dhan. `/instruments.json` defaults an unknown feed and that is a shipped
/// behaviour this does not copy: a page that mistypes a feed here would
/// otherwise enable four controls against a set the other broker reaches.
pub(super) async fn universe_reach_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = "application/json; charset=utf-8";
    let asked = param(uri.query().unwrap_or(""), "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // THE SENTENCE IS NOT WRITTEN HERE ANY MORE. It moved to
        // [`no_such_feed_json`] when `/store.json` and `/store` stopped
        // answering an unknown feed as Dhan and needed the same words.
        return (
            axum::http::StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, json)],
            no_such_feed_json(&asked),
        );
    };
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, json)],
        site.universe().read.coverage.json(feed),
    )
}

/// [`Read::status`](super::Read::status) — `ok` or `DEGRADED`, the same word
/// `/health` answers with.
pub const UNIVERSE_STATUS_HEADER: &str = "x-brutex-universe-status";

/// [`MasterState::name`] for the feed the request named.
pub const MASTER_STATE_HEADER: &str = "x-brutex-master-state";

/// The sentence behind [`MASTER_STATE_HEADER`].
pub const MASTER_NOTE_HEADER: &str = "x-brutex-master-note";

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
    use crate::server::tests::{
        DHAN_HEAD, agreeing, body_of, corrupt_census, get, masters, month_of, served_over, site,
        store_root,
    };
    use crate::server::{LEGACY_UNIVERSE_TOKENS, Site, UNIVERSE_TOKENS, universe_tokens};

    /// The rows [`census_on_disk`] records for a month at one rung.
    const ROWS: u64 = 375;

    /// Writes `vendor`'s single-file census under `root`, holding each
    /// `(segment, symbol, contract, rung, rows)` in January 2026.
    fn census_on_disk(
        root: &std::path::Path,
        vendor: Vendor,
        held: &[(Segment, &str, Option<&str>, store::path::Timeframe, u64)],
    ) {
        let mut manifest =
            pull::manifest::Manifest::open(vendor, &[], &[]).expect("a genesis census");
        for &(segment, symbol, contract, timeframe, rows) in held {
            manifest
                .record_held(pull::manifest::Held::new(
                    pull::manifest::Entry {
                        key: pull::manifest::EntryKey {
                            contract: contract.map(|text| {
                                brutex_core::instrument::Contract::parse(text).expect("a contract")
                            }),
                            exchange: Exchange::Nse,
                            segment,
                            symbol: Symbol::new(symbol).expect("a symbol"),
                            timeframe,
                            month: month_of(2026, 1),
                        },
                        rows,
                        first_ts_micros: 1_767_239_100_000_000,
                        last_ts_micros: 1_767_261_540_000_000,
                    },
                    pull::manifest::Closes::UNKNOWN,
                ))
                .expect("records");
        }
        std::fs::write(
            pull::manifest::manifest_path(root, vendor),
            manifest.image(),
        )
        .expect("a census on disk");
    }

    /// The rows of an `/instruments.json` body, by symbol and segment.
    fn rows_of(body: &str) -> Vec<serde_json::Value> {
        serde_json::from_str::<Vec<serde_json::Value>>(body).expect("a JSON array")
    }

    /// **EACH ROW COUNTS ROWS PER RUNG, AND `bars` IS ITS FINEST RUNG'S**
    /// (DB-042, D-3327 item 4).
    ///
    /// `bars` counted one-minute spot bars only, so a share held at the day
    /// rung alone read `0` and sorted last. RELIANCE here is held at `1day`
    /// only and must read its day rows; NIFTY at two rungs reads its minute
    /// rows; an option on NIFTY and NIFTY's BSE listing stay out of NSE
    /// NIFTY's count.
    #[test]
    fn each_row_counts_rows_per_rung_and_bars_is_its_finest_rungs() {
        let dir = agreeing("instruments-rungs");
        let built = site("instruments-rungs", &dir);
        census_on_disk(
            &built.store_root,
            Vendor::Dhan,
            &[
                (
                    Segment::Index,
                    "NIFTY",
                    None,
                    store::path::Timeframe::MINUTE_1,
                    ROWS,
                ),
                (
                    Segment::Index,
                    "NIFTY",
                    None,
                    store::path::Timeframe::DAY_1,
                    21,
                ),
                (
                    Segment::Cash,
                    "RELIANCE",
                    None,
                    store::path::Timeframe::DAY_1,
                    2_168,
                ),
                (
                    Segment::Fno,
                    "NIFTY",
                    Some("2026-01-29-2500000-CE"),
                    store::path::Timeframe::MINUTE_1,
                    9_999,
                ),
            ],
        );
        let (code, headers, body) = instruments_answer(&built, Vendor::Dhan);
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert_eq!(headers["x-brutex-census-state"], "held");
        let rows = rows_of(&body);
        let nifty = rows
            .iter()
            .find(|row| row["symbol"] == "NIFTY")
            .expect("NIFTY");
        assert_eq!(nifty["bars"], ROWS);
        assert_eq!(nifty["bars_rung"], "1min");
        assert_eq!(nifty["rows_by_rung"]["1day"], 21);
        assert_eq!(nifty["options"], true, "the feed holds options on it");
        assert_eq!(nifty["master"], "dhan");
        let reliance = rows
            .iter()
            .find(|row| row["symbol"] == "RELIANCE")
            .expect("RELIANCE");
        assert_eq!(reliance["bars"], 2_168, "a day-only share is not a zero");
        assert_eq!(reliance["bars_rung"], "1day");
        assert_eq!(reliance["options"], false);

        // KEPT PER CENSUS READ: the same census answers the same body, and a
        // census that moves answers the new count.
        let (_, _, again) = instruments_answer(&built, Vendor::Dhan);
        assert_eq!(again, body);
        census_on_disk(
            &built.store_root,
            Vendor::Dhan,
            &[(
                Segment::Cash,
                "RELIANCE",
                None,
                store::path::Timeframe::DAY_1,
                2_169,
            )],
        );
        let (_, _, moved) = instruments_answer(&built, Vendor::Dhan);
        let reliance = rows_of(&moved)
            .into_iter()
            .find(|row| row["symbol"] == "RELIANCE")
            .expect("RELIANCE");
        assert_eq!(reliance["bars"], 2_169);
    }

    /// **A FEED WITH NO MASTER IS LISTED FROM ITS CENSUS, AND NEVER AS "IN
    /// THIS MASTER"** (DB-041, D-3327 item 4).
    ///
    /// GDFL publishes no instrument master, so this route answered `[]` and
    /// every universe tier on /db was disabled. Its rows now come from the
    /// census: each spot series with its kind and its membership from the
    /// sourced lists, each option underlying with no spot series under `FNO`
    /// with `spot: false` and no kind, all of them `master: null`.
    #[test]
    fn a_feed_with_no_master_is_listed_from_its_census() {
        let dir = agreeing("instruments-gdfl");
        let built = site("instruments-gdfl", &dir);
        census_on_disk(
            &built.store_root,
            Vendor::Gdfl,
            &[
                (
                    Segment::Index,
                    "NIFTY",
                    None,
                    store::path::Timeframe::SECOND_1,
                    ROWS,
                ),
                (
                    Segment::Cash,
                    "RELIANCE",
                    None,
                    store::path::Timeframe::SECOND_1,
                    ROWS,
                ),
                (
                    Segment::Fno,
                    "NIFTY",
                    Some("2026-01-29-2500000-CE"),
                    store::path::Timeframe::SECOND_1,
                    ROWS,
                ),
                (
                    Segment::Fno,
                    "ALBK",
                    Some("2026-01-29-1500-PE"),
                    store::path::Timeframe::SECOND_1,
                    ROWS,
                ),
            ],
        );
        let (code, headers, body) = instruments_answer(&built, Vendor::Gdfl);
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert_eq!(
            headers[MASTER_STATE_HEADER], "not-mastered",
            "GDFL publishes no master"
        );
        let rows = rows_of(&body);
        assert_eq!(rows.len(), 3, "{body}");
        for row in &rows {
            assert_eq!(row["master"], serde_json::Value::Null, "{row}");
        }
        let nifty = rows
            .iter()
            .find(|row| row["symbol"] == "NIFTY")
            .expect("NIFTY");
        assert_eq!(nifty["kind"], "Index");
        assert_eq!(nifty["bars_rung"], "1s");
        assert_eq!(nifty["options"], true);
        assert!(
            nifty["universes"]
                .as_array()
                .expect("a list")
                .contains(&serde_json::json!("index")),
            "{nifty}"
        );
        let reliance = rows
            .iter()
            .find(|row| row["symbol"] == "RELIANCE")
            .expect("RELIANCE");
        assert_eq!(reliance["kind"], "Equity");
        assert!(
            reliance["universes"]
                .as_array()
                .expect("a list")
                .contains(&serde_json::json!("fno")),
            "RELIANCE is an F&O underlying by the sourced list: {reliance}"
        );
        let albk = rows
            .iter()
            .find(|row| row["symbol"] == "ALBK")
            .expect("ALBK");
        assert_eq!(albk["spot"], false);
        assert_eq!(
            albk["kind"],
            serde_json::Value::Null,
            "no spot series, no kind"
        );
        assert_eq!(albk["segment"], "FNO");
        assert_eq!(albk["bars"], 0);
        assert_eq!(albk["options"], true);
    }

    /// **`/instruments.json` REFUSES A FEED IT DOES NOT READ, RATHER THAN
    /// ANSWERING AS DHAN.**
    ///
    /// The last of twelve `parse_vendor` call sites in this crate that
    /// substituted. `ingest::parse_vendor` answers the EMPTY string with Dhan
    /// itself, so the `.unwrap_or(Vendor::Dhan)` that stood here was
    /// unreachable for an absent `?feed=` and caught exactly one input: a feed
    /// somebody NAMED and this build does not read.
    ///
    /// Everything downstream then became Dhan's answer under **HTTP 200** — the
    /// row filter, the per-symbol bar counts, and the census and master
    /// headers. `masters_json` removed the identical line and said why: being
    /// served one vendor's copy after asking for another's is a wrong answer
    /// wearing the shape of a right one.
    ///
    /// Both arms are asserted because only the pair proves the rule: an absent
    /// feed must still work, or "refuse the unknown" would have quietly broken
    /// every page that omits the parameter.
    #[tokio::test]
    async fn instruments_json_refuses_a_named_feed_it_cannot_read() {
        let dir = masters("feedrefuse", None, None);
        let site = Site::serving(&dir, &store_root("feedrefuse"));
        let loaded = std::sync::Arc::new(site);

        let (status, _headers, body) = instruments_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/instruments.json?feed=notafeed".parse().expect("a uri"),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "a feed this build cannot read is refused, never answered as \
             another vendor's list: {body}"
        );
        assert!(
            body.contains("notafeed"),
            "and the refusal names what was asked for: {body}"
        );

        // AN ABSENT FEED IS NOT AN UNKNOWN ONE. `parse_vendor` answers the
        // empty string with the descriptor's default, so every page that omits
        // the parameter must still be served — without this half, "refuse the
        // unknown" would have broken all of them.
        let (status, _headers, body) = instruments_json(
            axum::extract::State(loaded),
            "/instruments.json".parse().expect("a uri"),
        )
        .await;
        assert_ne!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "an absent feed resolves to the descriptor's default and must NOT \
             take the unknown-feed arm: {body}"
        );
        assert!(
            !body.contains("refused"),
            "and it is not answered with the no-such-feed body: {body}"
        );
        // NOT asserted as 200: this fixture store holds no census, so the route
        // answers 503 for a reason that has nothing to do with the feed. What
        // matters here is only that it did not take the refusal arm — pinning
        // the success code would be testing the fixture, not the rule.
    }

    /// **`/instruments.json` used to answer both of its failures with `200`
    /// and nothing else.**
    ///
    /// Two states, one route, and neither reached the caller:
    ///
    /// * the counter would not load, so `rows_for` answered `None` for every
    ///   key and every row read `"bars":0` — the same em dash an un-pulled
    ///   instrument shows;
    /// * the selected feed's master would not decode, so no row carried that
    ///   feed's id, the filter admitted nothing, and the body was `[]` —
    ///   identical to a feed that lists nothing.
    ///
    /// `Read::notes` and `Read::status` were one field away, and `/health`
    /// answers `503` off exactly them. Both are on the wire now, and the second
    /// case gets the per-feed answer a whole-read boolean could never give:
    /// `?feed=dhan` on the very same site is `200` and complete.
    #[tokio::test]
    async fn instruments_json_says_when_its_zeroes_are_not_measurements() {
        // (a) THE COUNTER IS DAMAGED. The universe is intact, so the LIST is
        // real and only the counts are absent — 503, and the census headers say
        // which half is not to be believed.
        let dir = agreeing("instrcensus");
        let root = corrupt_census("instrcensus");
        let built = Loaded::new(Site::load(&dir, &root));
        served_over("instrcensus", built, |addr| async move {
            let answer = get(addr, "/instruments.json?feed=groww").await;
            assert!(answer.starts_with("HTTP/1.1 503"), "{answer}");
            assert!(
                answer.contains("x-brutex-census-state: unreadable"),
                "{answer}"
            );
            assert!(
                answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
                "the reason, not just a code: {answer}"
            );
            assert!(
                answer.contains("x-brutex-master-state: read"),
                "the master decoded — it is the counter that did not: {answer}"
            );
            // THE ZEROES ARE STILL THERE AND STILL AN ARRAY. Nothing about the
            // body moved; what moved is that the response now says the zeroes
            // are absent rather than measured.
            let body = body_of(&answer);
            assert!(body.starts_with('['), "still an array: {body}");
            assert!(body.contains(r#""bars":0"#), "{body}");
        })
        .await;

        // (b) THE SELECTED FEED'S MASTER NEVER DECODED. The body is `[]` and
        // that is not this feed listing nothing.
        let half = masters(
            "instrmaster",
            None,
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let built = Loaded::new(Site::load(&half, &store_root("instrmaster")));
        served_over("instrmaster", built, |addr| async move {
            let missing = get(addr, "/instruments.json?feed=groww").await;
            assert_eq!(body_of(&missing), "[]", "{missing}");
            assert!(
                missing.starts_with("HTTP/1.1 503"),
                "an empty list from a failed read is not an empty universe: {missing}"
            );
            assert!(
                missing.contains("x-brutex-master-state: UNAVAILABLE"),
                "{missing}"
            );
            assert!(
                missing.contains("groww_instruments.csv"),
                "the note names the file that was not read: {missing}"
            );
            assert!(
                missing.contains("x-brutex-universe-status: DEGRADED"),
                "the same word /health answers with: {missing}"
            );

            // AND THE OTHER FEED ON THE SAME SITE IS FINE. This is what a
            // whole-read boolean cannot say, and it is why `Read::master`
            // exists: one master failed, one did not, and the route answers per
            // feed.
            let ok = get(addr, "/instruments.json?feed=dhan").await;
            assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
            assert!(ok.contains("x-brutex-master-state: read"), "{ok}");
            assert!(
                body_of(&ok).contains(r#""symbol":"NIFTY""#),
                "and it lists what it read: {ok}"
            );
        })
        .await;
    }

    /// The type-ahead index is the SAME universe the page shows.
    ///
    /// It shipped `merged.by_key` whole — 2,780 listings — while the page
    /// beside it said 785. A suggestion the operator's own universe excludes is
    /// worse than no suggestion: the click lands on a page that does not list
    /// it and nothing says why. `CLAUDE.md` bounds the surface; a second
    /// endpoint that does not honour the bound is the bound not holding.
    #[tokio::test]
    async fn the_typeahead_index_is_the_tracked_universe_and_not_the_master() {
        let dir = agreeing("tajson");
        let site = site("tajson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);

        let (_code, _headers, json) = instruments_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            // The feed is part of the question now: the two brokers do not
            // list the same instruments, so the index is per feed.
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        let site = &*loaded;

        let shipped = json.matches(r#""symbol":"#).count();
        let tracked = site
            .universe()
            .read
            .merged
            .by_key
            .values()
            .filter(|e| crate::catalog::tracked(e.universe))
            .count();

        assert_eq!(
            shipped, tracked,
            "the index must carry exactly the tracked universe — not the whole \
             master, which is what shipped 2,780 rows behind a page saying 785"
        );
        assert!(
            shipped <= site.universe().read.merged.by_key.len(),
            "and it cannot exceed the master it is drawn from"
        );

        // The escaper is exercised on real data rather than asserted about:
        // `&` is a legal symbol byte (M&M, M&MFIN) and a raw control character
        // would make the whole document unparseable, losing the type-ahead
        // rather than one row.
        assert!(
            !json.contains('\n') && !json.contains('\t'),
            "no raw control characters reach the document"
        );
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "and it is an array"
        );
    }

    /// The wire says every universe a row is in, and the old field never moves.
    ///
    /// D-0089 landed NIFTY 50 / 100 / 200 / 500 as `Universe` bits 3–6 and
    /// wired none of it: `/instruments.json` still answered one word out of
    /// `index|fno|ntm|other`, so four rows on `/` and `/db` shipped DISABLED
    /// with "no endpoint carries this membership" written on them. The data was
    /// there and the API could not say it.
    ///
    /// Two fields now, and this test is the reason both exist. `universes`
    /// carries the set. `universe` is frozen at the three words a running page
    /// already splits on `+` — widening it would have been the quiet kind of
    /// break, because the string would still parse and would still group, and
    /// only a reader comparing or bookmarking the whole value would notice.
    ///
    /// Pinned on a value with a NEW bit and no old one — a bare `NIFTY_50` —
    /// because that is the only shape that can tell "the compatibility field is
    /// pinned to three" apart from "the new bits happen to travel with `ntm`",
    /// which every real constituent does.
    #[tokio::test]
    async fn the_wire_carries_every_membership_and_the_frozen_field_never_widens() {
        use brutex_core::universe::Universe;

        // Nothing at all: the array is empty, the word is `other`. The two
        // spellings of "no membership" are deliberate and are not equal.
        assert_eq!(universe_label(Universe::NONE), "other");
        assert!(universe_tokens(Universe::NONE).is_empty());
        assert_eq!(universes_json(Universe::NONE), "[]");

        // Everything: the frozen field is STILL exactly three words.
        let every = UNIVERSE_TOKENS
            .iter()
            .fold(Universe::NONE, |acc, (bit, _)| acc.union(*bit));
        assert_eq!(
            universe_label(every),
            "index+fno+ntm",
            "the compatibility field is pinned to the three universes that \
             existed before D-0089, whatever else the row is in"
        );
        assert_eq!(
            universe_tokens(every),
            ["index", "fno", "ntm", "n500", "n200", "n100", "n50"],
            "and the array carries all seven, in bit order"
        );

        // A new bit ALONE cannot reach the frozen field.
        assert_eq!(universe_label(Universe::NIFTY_50), "other");
        assert_eq!(universe_tokens(Universe::NIFTY_50), ["n50"]);
        assert_eq!(universes_json(Universe::NIFTY_50), r#"["n50"]"#);

        // And the two fields cannot disagree about how a bit is spelled,
        // because both read `UNIVERSE_TOKENS`.
        for (bit, token) in UNIVERSE_TOKENS.into_iter().take(LEGACY_UNIVERSE_TOKENS) {
            assert_eq!(universe_label(bit), token, "one spelling per bit");
        }

        // Now the endpoint, on real membership rather than synthesised bits.
        let dir = agreeing("unijson");
        let site = site("unijson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);
        let (_code, _headers, json) = instruments_json(
            axum::extract::State(loaded),
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;

        // RELIANCE is in all six equity lists — that is what the exchange's own
        // constituent files say, and it is why the four tiers were worth
        // wiring: `fno+ntm` was the whole answer this endpoint could give.
        assert!(
            json.contains(
                r#""universe":"fno+ntm","universes":["fno","ntm","n500","n200","n100","n50"]"#
            ),
            "RELIANCE must carry its four tiers in the array and none of them \
             in the frozen word: {json}"
        );
        // NIFTY is an index and an F&O underlying, and a constituent of
        // nothing: an index is not a share. Both fields say so.
        assert!(
            json.contains(r#""universe":"index+fno","universes":["index","fno"]"#),
            "an index names no tier: {json}"
        );
        // Every row carries both. `"universe":` is not a substring of
        // `"universes":` — the byte after `universe` differs — so these two
        // counts are independent.
        let frozen = json.matches(r#""universe":"#).count();
        let full = json.matches(r#""universes":"#).count();
        assert_eq!(frozen, json.matches(r#""symbol":"#).count());
        assert_eq!(full, frozen, "neither field is optional");
    }

    #[tokio::test]
    async fn universes_json_says_what_each_target_resolves_to_for_the_named_feed() {
        // THE ROUTE THE `/ingest` UNIVERSE MENU HAD NO ANSWER FROM. Its four
        // NIFTY rows drew `no target` because nothing on the wire turned a
        // tier into a request for the selected feed. D-0120.
        let dir = agreeing("reachjson");
        let loaded: Loaded = std::sync::Arc::new(site("reachjson", &dir));

        let (status, _headers, json) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            json.starts_with(r#"{"feed":"groww","mastered":true,"#),
            "{json}"
        );

        // ALL SEVEN ROWS, EACH WITH THE SLUG A POST WOULD CARRY. The four
        // tiers are the point: a page that reads this can enable them.
        for target in ingest::SpotTarget::ALL {
            let slug = target.slug();
            assert!(
                json.contains(&format!(r#""target":"{slug}""#)),
                "{slug} must be requestable from this answer: {json}"
            );
        }
        // RELIANCE is in every equity list this fixture's master carries, so
        // each tier resolves to exactly one Groww id and the rest of the
        // published list is named as unreachable rather than dropped.
        assert!(
            json.contains(r#""target":"n50","label":"NIFTY 50 equities""#),
            "{json}"
        );
        assert!(
            json.contains(
                r#""counted_from":"join","published":50,"matched":1,"lacks":49,"ambiguous":0,"malformed":0,"no_nse_isin":0"#
            ),
            "the count on the control is the matched bucket and the shortfall is \
             beside it, not behind it: {json}"
        );
        // AND THE BUCKET IS THE RIGHT ONE, WHICH D-0125 CHANGED. The join is
        // keyed on NSE's OWN ISIN now, so every NIFTY 50 name HAS an identity
        // whether or not a master mentions it — the forty-nine this fixture's
        // master does not carry are `lacks`, each naming the ISIN the exchange
        // printed, and `malformed` is empty because no published name here is
        // unusable. The two are still not interchangeable: `lacks` is a row to
        // go and ask Groww about, `malformed` would be a transcription to fix
        // in this repository, and `no_nse_isin` is a cell NSE left empty.
        assert!(
            json.contains(
                r#""symbol":"3MINDIA","bucket":"lacks","why":"this feed's master carries no row for ISIN INE470A01017""#
            ),
            "{json}"
        );
        assert!(
            json.contains(r#""counted_from":"master","published":null,"matched":1,"lacks":0"#),
            "and the two targets no published list defines say where THEIR count \
             came from rather than reporting a denominator nobody published: {json}"
        );

        // THE FEED DECIDES THE ANSWER. Dhan's master in this fixture carries
        // the same two instruments, so the counts match and the ids do not —
        // what must differ is the feed the document is about.
        let (status, _headers, other) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=dhan".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(other.starts_with(r#"{"feed":"dhan","#), "{other}");

        // AN ARCHIVE FEED IS NOT AN EMPTY ONE. A folder of CSVs is its own
        // listing and publishes no master, so every count is null.
        let (status, _headers, archive) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=truedata"
                .parse()
                .expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            archive.contains(r#""mastered":false"#)
                && archive.contains(r#""counted_from":"no master""#),
            "{archive}"
        );

        // AND AN UNKNOWN FEED IS REFUSED BY NAME. `/instruments.json` answers
        // an unrecognised feed as Dhan; copying that here would silently
        // enable four controls against the other broker's reach.
        //
        // `zerodha` WAS THIS EXAMPLE until 14 Aug 2026, when it became a feed
        // this build names. A test for an unknown feed has to name one that
        // stays unknown, or it stops testing the refusal and starts testing
        // whichever vendor was added last.
        let (status, _headers, refused) = universe_reach_json(
            axum::extract::State(loaded),
            "/universes.json?feed=upstox".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            refused.contains(r#""feed":"upstox""#)
                && refused.contains("this build reads no feed called that")
                && refused.contains("groww")
                && refused.contains("dhan"),
            "the refusal names what was asked and what is known: {refused}"
        );
    }
}
