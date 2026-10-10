//! `/census/facets.json` -- the selection-scoped lists the /db pickers need,
//! answered from an index kept per (feed, segment) over the cached census,
//! with `?since=<generation>` deltas. D-3327 item 3; DB-060, DB-G59, DB-G70.
//!
//! # Why this route exists
//!
//! /db built every picker (underlyings, expiries, strikes, months, counts)
//! by downloading the WHOLE census of the feed and folding it in the
//! browser: 21 MB and 0.7 s for GDFL on 6 Oct 2026, about 1.3 s of a frozen
//! main thread, and a cost that grows with every month the options import
//! adds (about 180 MB once the full archive is counted, an extrapolation).
//! A selection needs a few hundred rows of that. This answers exactly the
//! selection's rows.
//!
//! ```text
//! GET /census/facets.json?feed=gdfl&segment=FNO                          underlyings
//! GET /census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY         + months, expiries
//! GET /census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28
//!                                                                         + strikes, futures, the chain's months
//! GET /census/facets.json?...&since=<generation>                          only what changed after it
//! ```
//!
//! # Map probes, not a walk
//!
//! The index ([`Index`]) holds, per underlying, its months and expiries, and
//! per expiry its contracts, each with rows per rung, first and last stamp
//! and last close summed up the tree. It is built once from a census (one
//! pass over that segment's keys) and then MOVED FORWARD by the entries the
//! census appended since ([`census::Part::changed_since`]): each changed key
//! adjusts the nodes on its own path by its new row minus the row the index
//! last saw for it. A request then reads the nodes its selection names, so
//! it costs the size of its answer, whatever the census holds.
//!
//! # What `since` answers
//!
//! The rows whose census entry was appended after that generation, with
//! their current totals, so a client merges them by key. A generation the
//! census no longer holds the appends of (it was read whole since: a new
//! epoch, a compaction, a restart) is refused `410 since_too_old`, and the
//! client loads the selection afresh; never a silent partial delta.
//!
//! # Cost
//!
//! A request: the per-vendor census read (`census_cache::vendor_census`), the
//! index's lock, the appended entries since the index last moved (O(log)
//! each), and the answer. A cold index is one pass over the segment's keys,
//! once per census read whole. Measured on a shadow of the operator's store
//! (see the package report): the figures are not restated here.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{EntryKey, Held};
use pull::session::Day;
use store::path::{Timeframe, YearMonth};

use super::census_cache::Slot;
use super::{Loaded, Site, no_such_feed_json, param, vendor_census};
use crate::{census, ingest, render};

/// Rows per rung, finest rung first ([`Timeframe`]'s order is its length).
type Rows = BTreeMap<Timeframe, u64>;

/// What one node of the index sums: rows per rung, the first and last stamp,
/// and the close of the newest entry under it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Agg {
    /// Rows per rung.
    pub(crate) rows: Rows,
    /// The earliest first stamp, in microseconds.
    pub(crate) first: Option<i64>,
    /// The latest last stamp, in microseconds.
    pub(crate) last: Option<i64>,
    /// The latest last stamp seen with a close, and that close in paisa (or
    /// `None` where the census recorded none for it).
    close: Option<(i64, Option<i64>)>,
}

impl Agg {
    /// One entry moved from `old` (or new) to `new`.
    ///
    /// Rows move by the difference, exactly: the census only lets a key's
    /// rows grow (`ManifestError::RowCountWentBackwards`). The last stamp
    /// only grows too (`KeyTimestampsOutOfOrder`), so a maximum is exact. The
    /// first stamp is a minimum: a month's first bar is fixed once written
    /// (bar files are append-only, `store::file`), so a key's first stamp
    /// does not move after its first entry.
    fn apply(&mut self, old: Option<&Held>, new: &Held) {
        let entry = &new.entry;
        let rows = self.rows.entry(entry.key.timeframe).or_insert(0);
        *rows = rows
            .saturating_sub(old.map_or(0, |was| was.entry.rows))
            .saturating_add(entry.rows);
        self.first = Some(self.first.map_or(entry.first_ts_micros, |first| {
            first.min(entry.first_ts_micros)
        }));
        self.last = Some(
            self.last
                .map_or(entry.last_ts_micros, |last| last.max(entry.last_ts_micros)),
        );
        if self.close.is_none_or(|(at, _)| entry.last_ts_micros >= at) {
            self.close = Some((entry.last_ts_micros, new.closes.last_paisa()));
        }
    }

    /// The close of the newest entry under this node, in paisa.
    pub(crate) fn last_close(&self) -> Option<i64> {
        self.close.and_then(|(_, close)| close)
    }
}

/// Which side of an option, or a future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ContractId {
    /// `<expiry>-FUT`.
    Future,
    /// `<expiry>-<strike paisa>-CE|PE`; `call` is true for `CE`.
    Option { strike: i64, call: bool },
}

/// A contract's expiry and its id, from the store's own spelling
/// (`brutex_core::instrument::Contract`): `YYYY-MM-DD-FUT` or
/// `YYYY-MM-DD-<paisa>-CE|PE`. `None` for any other text, which is counted,
/// never guessed at.
fn contract_of(text: &str) -> Option<(Day, ContractId)> {
    let (date, rest) = text.split_at_checked(10)?;
    let rest = rest.strip_prefix('-')?;
    let mut parts = date.split('-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    let expiry = Day::new(year, month, day).ok()?;
    if rest == "FUT" {
        return Some((expiry, ContractId::Future));
    }
    let (strike, side) = rest.rsplit_once('-')?;
    let call = match side {
        "CE" => true,
        "PE" => false,
        _ => return None,
    };
    let strike: i64 = strike.parse().ok()?;
    Some((expiry, ContractId::Option { strike, call }))
}

/// One expiry's chain.
#[derive(Debug, Clone, Default)]
struct Expiry {
    agg: Agg,
    months: BTreeMap<YearMonth, Agg>,
    contracts: BTreeMap<ContractId, Agg>,
}

/// One underlying: its months and, in FNO, its expiries.
#[derive(Debug, Clone, Default)]
pub(crate) struct Underlying {
    pub(crate) agg: Agg,
    months: BTreeMap<YearMonth, Agg>,
    expiries: BTreeMap<Day, Expiry>,
    /// Keys whose contract text is not one this index can read.
    unread_contracts: u64,
}

/// The index for one (feed, segment), as of one census read. See the module
/// header.
#[derive(Debug, Clone, Default)]
pub(crate) struct Index {
    /// The census read these sums equal.
    reflects: Option<Arc<Slot>>,
    /// The generation of the part that holds this segment, as of `reflects`.
    generation: u64,
    underlyings: BTreeMap<(Exchange, Symbol), Underlying>,
}

impl Index {
    /// Every underlying, in exchange and symbol order.
    pub(crate) fn underlyings(&self) -> impl Iterator<Item = (&(Exchange, Symbol), &Underlying)> {
        self.underlyings.iter()
    }

    /// One key moved from `old` to `new`.
    fn apply(&mut self, old: Option<&Held>, new: &Held) {
        let key = new.entry.key;
        let under = self
            .underlyings
            .entry((key.exchange, key.symbol))
            .or_default();
        under.agg.apply(old, new);
        under.months.entry(key.month).or_default().apply(old, new);
        let Some(contract) = key.contract else {
            return;
        };
        let Some((expiry, id)) = contract_of(contract.as_str()) else {
            if old.is_none() {
                under.unread_contracts = under.unread_contracts.saturating_add(1);
            }
            return;
        };
        let chain = under.expiries.entry(expiry).or_default();
        chain.agg.apply(old, new);
        chain.months.entry(key.month).or_default().apply(old, new);
        chain.contracts.entry(id).or_default().apply(old, new);
    }
}

/// The part of `slot`'s census that holds `segment`'s keys, if it loaded.
fn part_of(slot: &Slot, segment: Segment) -> Option<&census::Part> {
    match slot.census().state {
        census::Census::Held { ref manifest } => manifest.part_of(segment),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    }
}

/// One index per (feed, segment), each behind its own lock.
#[derive(Debug, Default)]
pub(super) struct Cache(Mutex<HashMap<(Vendor, Segment), IndexCell>>);

/// One (feed, segment)'s index, behind the lock its moves are made under.
type IndexCell = Arc<Mutex<Index>>;

/// `segment`'s index as of `slot`, built or moved forward, under its lock.
///
/// Moved forward when the index reflects an earlier read of the same census
/// and that census still holds the appends since (`changed_since`); built
/// from the census otherwise. Either way the sums equal a fresh build over
/// `slot`'s census: `facets::a_moved_index_equals_a_fresh_build` holds it.
pub(crate) fn index<T>(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    slot: &Arc<Slot>,
    read: impl FnOnce(&Index) -> T,
) -> T {
    let cell = {
        let mut cells = site
            .census_wire
            .facets
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(cells.entry((vendor, segment)).or_default())
    };
    let mut held = cell.lock().unwrap_or_else(PoisonError::into_inner);
    advance(&mut held, segment, slot);
    read(&held)
}

/// [`index`]'s update, on its own so a test drives it without a site.
fn advance(index: &mut Index, segment: Segment, slot: &Arc<Slot>) {
    if index
        .reflects
        .as_ref()
        .is_some_and(|reflects| Arc::ptr_eq(reflects, slot))
    {
        return;
    }
    let Some(part) = part_of(slot, segment) else {
        *index = Index {
            reflects: Some(Arc::clone(slot)),
            ..Index::default()
        };
        return;
    };
    // MOVED FORWARD ONLY ALONG ONE READ OF ONE FILE: the parts must share
    // their whole read, or the generation the index stands at may name a
    // census of another file (a new epoch, a migration, a whole re-read).
    let moved = index.reflects.as_ref().and_then(|before| {
        let before = part_of(before, segment)?;
        if !part.extends_same_read(before) {
            return None;
        }
        let changes = part.changed_since(index.generation)?;
        Some((before.clone(), changes))
    });
    if let Some((before, changes)) = moved {
        let mut seen: HashSet<EntryKey> = HashSet::with_capacity(changes.len());
        for (_, key) in changes.iter() {
            if key.segment != segment || !seen.insert(*key) {
                continue;
            }
            if let Some(now) = part.held(key) {
                index.apply(before.held(key).as_ref(), &now);
            }
        }
    } else {
        let mut fresh = Index::default();
        for key in part.held_keys() {
            if key.segment != segment {
                continue;
            }
            if let Some(held) = part.held(key) {
                fresh.apply(None, &held);
            }
        }
        *index = fresh;
    }
    index.reflects = Some(Arc::clone(slot));
    index.generation = part.header().generation;
}

/// What a request asked for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Asked {
    feed: Vendor,
    segment: Segment,
    underlying: Option<Symbol>,
    exchange: Option<Exchange>,
    expiry: Option<Day>,
    since: Option<u64>,
}

/// Reads the query, or names the parameter it cannot read.
fn asked(query: &str) -> Result<Asked, (axum::http::StatusCode, String)> {
    let bad = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            error_json("bad_request", &why),
        )
    };
    let feed_word = param(query, "feed");
    let Some(feed) = (!feed_word.is_empty())
        .then(|| ingest::parse_vendor(&feed_word))
        .flatten()
    else {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            no_such_feed_json(&feed_word),
        ));
    };
    let segment_word = param(query, "segment");
    let segment = Segment::parse(&segment_word).map_err(|why| {
        bad(format!(
            "segment {segment_word:?}: {why}; one of INDEX, CASH, FNO is required"
        ))
    })?;
    let underlying = match param(query, "underlying").as_str() {
        "" => None,
        word => Some(Symbol::new(word).map_err(|why| bad(format!("underlying {word:?}: {why}")))?),
    };
    let exchange = match param(query, "exchange").as_str() {
        "" => None,
        word => {
            Some(Exchange::parse(word).map_err(|why| bad(format!("exchange {word:?}: {why}")))?)
        }
    };
    let expiry = match param(query, "expiry").as_str() {
        "" => None,
        word => {
            let (expiry, _) = contract_of(&format!("{word}-FUT"))
                .ok_or_else(|| bad(format!("expiry {word:?} is not a YYYY-MM-DD day")))?;
            if underlying.is_none() || segment != Segment::Fno {
                return Err(bad("expiry needs segment=FNO and an underlying".to_owned()));
            }
            Some(expiry)
        }
    };
    let since = match param(query, "since").as_str() {
        "" => None,
        word => Some(
            word.parse::<u64>()
                .map_err(|_| bad(format!("since {word:?} is not a generation")))?,
        ),
    };
    Ok(Asked {
        feed,
        segment,
        underlying,
        exchange,
        expiry,
        since,
    })
}

/// `{"error": <code>, "why": <sentence>}`.
fn error_json(code: &str, why: &str) -> String {
    format!(
        r#"{{"error":{},"why":{}}}"#,
        render::json_string(code),
        render::json_string(why)
    )
}

/// `GET /census/facets.json`. See the module header.
pub(super) async fn facets_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let query = uri.query().unwrap_or("").to_owned();
    // OFF THE ASYNC WORKERS (DB-066, DB-G80): a census miss reads a manifest,
    // and a cold index walks a segment's keys. A full pool is a named 429.
    match crate::detail::run_store_read(move || answer(&site, &query)).await {
        Ok(answered) => answered,
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
                error_json("census_read_unavailable", &format!("{why:?}")),
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

/// The whole answer, off the async workers.
pub(super) fn answer(
    site: &Site,
    query: &str,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let asked = match asked(query) {
        Ok(asked) => asked,
        Err((code, body)) => return (code, json_headers(), body),
    };
    let (_stamps, now) = vendor_census(site, asked.feed);
    let census = now.census();
    let behind = super::store_route::behind(site, asked.feed);
    let headers = super::store_route::census_headers_with(census, Some(&behind));
    let state = census.map_or("absent", |census| census.state_word(behind.days));
    if super::store_route::census_is_unreadable(census) {
        let why = census.map_or_else(String::new, census::VendorCensus::note);
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            error_json("census_unreadable", &why),
        );
    }
    // WHAT LOADING STEPPED OVER, IN THE BODY AS WELL AS THE HEADER: a
    // census recovered from an older generation answers with fewer rows than
    // were committed, and a reader of the lists alone must be able to say so.
    let degraded = census.and_then(census::VendorCensus::degraded);
    let slot = now.slot();
    let body = index(site, asked.feed, asked.segment, slot, |index| {
        render_answer(index, &asked, state, degraded.as_deref())
    });
    match body {
        Ok(body) => (axum::http::StatusCode::OK, headers, body),
        Err((code, body)) => (code, headers, body),
    }
}

/// The selection's lists out of one index, or the refusal of a `since` it
/// cannot answer, or of an underlying two exchanges hold.
#[expect(
    clippy::too_many_lines,
    reason = "one answer written in one pass, list by list, in the order the wire carries \
              them; each list reads only the nodes its selection names"
)]
fn render_answer(
    index: &Index,
    asked: &Asked,
    state: &str,
    degraded: Option<&str>,
) -> Result<String, (axum::http::StatusCode, String)> {
    // WHAT CHANGED, when a delta was asked: the keys appended after `since`,
    // in this segment. `None` is "every row", the cold answer.
    let changed: Option<Changed> = match asked.since {
        None => None,
        Some(since) => {
            let Some(part) = index
                .reflects
                .as_ref()
                .and_then(|slot| part_of(slot, asked.segment))
            else {
                return Err((
                    axum::http::StatusCode::GONE,
                    error_json(
                        "since_too_old",
                        "this feed has no census for the segment, so no generation is held",
                    ),
                ));
            };
            let Some(changes) = part.changed_since(since) else {
                return Err((
                    axum::http::StatusCode::GONE,
                    format!(
                        r#"{{"error":"since_too_old","why":{},"since":{since},"floor":{},"generation":{}}}"#,
                        render::json_string(
                            "the census no longer holds the appends after that generation \
                             (it was read whole since, or the generation is not one it \
                             published); load the selection afresh"
                        ),
                        part.since_floor(),
                        index.generation
                    ),
                ));
            };
            Some(Changed::of(changes, asked.segment))
        }
    };
    // THE ASKED UNDERLYING BY PROBE: one map lookup per exchange the store
    // keys on, never a walk of the underlyings.
    let chosen: Vec<(&(Exchange, Symbol), &Underlying)> = match asked.underlying {
        None => Vec::new(),
        Some(symbol) => [Exchange::Nse, Exchange::Bse]
            .into_iter()
            .filter(|exchange| asked.exchange.is_none_or(|wanted| wanted == *exchange))
            .filter_map(|exchange| index.underlyings.get_key_value(&(exchange, symbol)))
            .collect(),
    };
    if chosen.len() > 1 {
        let exchanges: Vec<&str> = chosen
            .iter()
            .map(|((exchange, _), _)| exchange.as_str())
            .collect();
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            error_json(
                "ambiguous_underlying",
                &format!(
                    "{} is held on {}; name one with exchange=",
                    asked
                        .underlying
                        .as_ref()
                        .map_or("", |symbol| symbol.as_str()),
                    exchanges.join(", ")
                ),
            ),
        ));
    }
    let wanted =
        |id: &(Exchange, Symbol)| changed.as_ref().is_none_or(|c| c.underlyings.contains(id));

    let mut out = format!(
        r#"{{"schema":1,"feed":{},"segment":{},"underlying":{},"expiry":{},"generation":{},"since":{},"state":{},"degraded":{},"underlyings":["#,
        render::json_string(asked.feed.as_str()),
        render::json_string(asked.segment.as_str()),
        asked.underlying.map_or_else(
            || "null".to_owned(),
            |symbol| render::json_string(symbol.as_str())
        ),
        asked.expiry.map_or_else(
            || "null".to_owned(),
            |day| render::json_string(&day.to_string())
        ),
        index.generation,
        asked
            .since
            .map_or_else(|| "null".to_owned(), |since| since.to_string()),
        render::json_string(state),
        degraded.map_or_else(|| "null".to_owned(), render::json_string),
    );
    // THE UNDERLYINGS: all of them with no underlying asked, or the one.
    let listed: Vec<(&(Exchange, Symbol), &Underlying)> = if asked.underlying.is_none() {
        index.underlyings().filter(|(id, _)| wanted(id)).collect()
    } else {
        chosen
            .iter()
            .copied()
            .filter(|(id, _)| wanted(id))
            .collect()
    };
    for (n, ((exchange, symbol), under)) in listed.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let contracts: usize = under
            .expiries
            .values()
            .map(|chain| chain.contracts.len())
            .sum();
        let _ = write!(
            out,
            r#"{{"exchange":{},"symbol":{},"months":{},"expiries":{},"contracts":{contracts},"unread_contracts":{},"#,
            render::json_string(exchange.as_str()),
            render::json_string(symbol.as_str()),
            under.months.len(),
            under.expiries.len(),
            under.unread_contracts,
        );
        agg_json(&mut out, &under.agg, asked.segment != Segment::Fno);
        out.push('}');
    }
    out.push_str(r#"],"months":["#);
    let mut expiries_json = String::new();
    let mut strikes_json = String::new();
    let mut futures_json = String::new();
    if let Some(&(id, under)) = chosen.first() {
        let changed_months = changed.as_ref().map(|c| c.months_of(id, asked.expiry));
        // THE MONTHS: the underlying's, or the asked chain's.
        let chain = asked.expiry.and_then(|expiry| under.expiries.get(&expiry));
        let months = match (asked.expiry, chain) {
            (None, _) => Some(&under.months),
            (Some(_), Some(chain)) => Some(&chain.months),
            (Some(_), None) => None,
        };
        for (n, (month, agg)) in months
            .into_iter()
            .flatten()
            .filter(|(month, _)| {
                changed_months
                    .as_ref()
                    .is_none_or(|set| set.contains(month))
            })
            .enumerate()
        {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                r#"{{"month":{},"#,
                render::json_string(&month.to_string())
            );
            agg_json(
                &mut out,
                agg,
                asked.segment != Segment::Fno || asked.expiry.is_some(),
            );
            out.push('}');
        }
        if asked.expiry.is_none() {
            let changed_expiries = changed.as_ref().map(|c| c.expiries_of(id));
            for (n, (expiry, chain)) in under
                .expiries
                .iter()
                .filter(|(expiry, _)| {
                    changed_expiries
                        .as_ref()
                        .is_none_or(|set| set.contains(expiry))
                })
                .enumerate()
            {
                if n > 0 {
                    expiries_json.push(',');
                }
                let strikes: BTreeSet<i64> = chain
                    .contracts
                    .keys()
                    .filter_map(|id| match *id {
                        ContractId::Option { strike, .. } => Some(strike),
                        ContractId::Future => None,
                    })
                    .collect();
                let _ = write!(
                    expiries_json,
                    r#"{{"expiry":{},"weekday":{},"contracts":{},"strikes":{},"future":{},"#,
                    render::json_string(&expiry.to_string()),
                    render::json_string(weekday(*expiry)),
                    chain.contracts.len(),
                    strikes.len(),
                    chain.contracts.contains_key(&ContractId::Future),
                );
                agg_json(&mut expiries_json, &chain.agg, false);
                expiries_json.push('}');
            }
        }
        if let (Some(expiry), Some(chain)) = (asked.expiry, chain) {
            let changed_ids = changed.as_ref().map(|c| c.contracts_of(id, expiry));
            let keep = |contract: &ContractId| {
                changed_ids
                    .as_ref()
                    .is_none_or(|set| set.contains(contract))
            };
            // ONE ROW PER STRIKE, its CE and PE beside each other.
            let mut by_strike: BTreeMap<i64, [Option<&Agg>; 2]> = BTreeMap::new();
            for (contract, agg) in &chain.contracts {
                match *contract {
                    ContractId::Option { strike, call } if keep(contract) => {
                        let sides = by_strike.entry(strike).or_default();
                        if let Some(side) = sides.get_mut(usize::from(!call)) {
                            *side = Some(agg);
                        }
                    }
                    ContractId::Future if keep(contract) => {
                        let _ = write!(
                            futures_json,
                            r#"{{"contract":{},"#,
                            render::json_string(&format!("{expiry}-FUT"))
                        );
                        agg_json(&mut futures_json, agg, true);
                        futures_json.push('}');
                    }
                    ContractId::Option { .. } | ContractId::Future => {}
                }
            }
            for (n, (strike, [ce, pe])) in by_strike.iter().enumerate() {
                if n > 0 {
                    strikes_json.push(',');
                }
                let _ = write!(strikes_json, r#"{{"strike":{strike},"ce":"#);
                side_json(&mut strikes_json, *ce);
                strikes_json.push_str(r#","pe":"#);
                side_json(&mut strikes_json, *pe);
                strikes_json.push('}');
            }
        }
    }
    let _ = write!(
        out,
        r#"],"expiries":[{expiries_json}],"strikes":[{strikes_json}],"futures":[{futures_json}]}}"#
    );
    Ok(out)
}

/// One option side's row, or `null` where the strike has no such side.
fn side_json(out: &mut String, agg: Option<&Agg>) {
    let Some(agg) = agg else {
        out.push_str("null");
        return;
    };
    out.push('{');
    agg_json(out, agg, true);
    out.push('}');
}

/// An [`Agg`]'s fields, without braces: `"rows":{…},"first_ts":…,"last_ts":…`
/// and, where `close` asks for it, `"last_close":…`. A close is shown only
/// where it is ONE series' close: a spot underlying, a month, a contract. An
/// option chain's newest close is some strike's, and is not shown as the
/// underlying's.
fn agg_json(out: &mut String, agg: &Agg, close: bool) {
    out.push_str(r#""rows":{"#);
    for (n, (rung, rows)) in agg.rows.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}:{rows}", render::json_string(rung.as_str()));
    }
    let number = |value: Option<i64>| value.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let _ = write!(
        out,
        r#"}},"first_ts":{},"last_ts":{}"#,
        number(agg.first),
        number(agg.last)
    );
    if close {
        let _ = write!(out, r#","last_close":{}"#, number(agg.last_close()));
    }
}

/// The weekday of a day: 1970-01-01 was a Thursday.
fn weekday(day: Day) -> &'static str {
    const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    NAMES
        .get(usize::try_from((i64::from(day.days_from_epoch()) + 3).rem_euclid(7)).unwrap_or(0))
        .copied()
        .unwrap_or("")
}

/// The keys a delta covers, by the index paths they touch.
#[derive(Debug, Default)]
struct Changed {
    underlyings: HashSet<(Exchange, Symbol)>,
    keys: Vec<EntryKey>,
}

impl Changed {
    fn of(changes: census::Since<'_>, segment: Segment) -> Self {
        let mut out = Self::default();
        let mut seen: HashSet<EntryKey> = HashSet::with_capacity(changes.len());
        for (_, key) in changes.iter() {
            if key.segment == segment && seen.insert(*key) {
                let _ = out.underlyings.insert((key.exchange, key.symbol));
                out.keys.push(*key);
            }
        }
        out
    }

    fn mine<'a>(&'a self, id: &'a (Exchange, Symbol)) -> impl Iterator<Item = &'a EntryKey> + 'a {
        self.keys
            .iter()
            .filter(move |key| (key.exchange, key.symbol) == *id)
    }

    fn months_of(&self, id: &(Exchange, Symbol), expiry: Option<Day>) -> BTreeSet<YearMonth> {
        self.mine(id)
            .filter(|key| {
                expiry.is_none_or(|wanted| {
                    key.contract
                        .and_then(|contract| contract_of(contract.as_str()))
                        .is_some_and(|(day, _)| day == wanted)
                })
            })
            .map(|key| key.month)
            .collect()
    }

    fn expiries_of(&self, id: &(Exchange, Symbol)) -> BTreeSet<Day> {
        self.mine(id)
            .filter_map(|key| {
                key.contract
                    .and_then(|contract| contract_of(contract.as_str()))
            })
            .map(|(day, _)| day)
            .collect()
    }

    fn contracts_of(&self, id: &(Exchange, Symbol), expiry: Day) -> BTreeSet<ContractId> {
        self.mine(id)
            .filter_map(|key| {
                key.contract
                    .and_then(|contract| contract_of(contract.as_str()))
            })
            .filter(|(day, _)| *day == expiry)
            .map(|(_, contract)| contract)
            .collect()
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
#[path = "facets_tests.rs"]
mod tests;
