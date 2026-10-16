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
//! GET /census/facets.json?feed=gdfl                                      the feed's segments (the root)
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

/// Every exchange, in the order a census sorts a series by (the type's own
/// `Ord`). The answer states it (`order`), so no reader keeps a copy of this
/// list: WP-56b, the WP-56 second agent's follow-up 6.
/// `facets::the_order_stated_is_the_types_own_and_names_every_variant` holds
/// it to the type.
pub(crate) const EXCHANGES: [Exchange; 2] = [Exchange::Nse, Exchange::Bse];

/// Every segment, in the order a census sorts a series by. See [`EXCHANGES`].
pub(crate) const SEGMENTS: [Segment; 3] = [Segment::Index, Segment::Cash, Segment::Fno];

/// `{"exchanges":[..],"segments":[..]}`: the order the census lists a series
/// in, as every answer states it.
fn order_json() -> String {
    let words = |list: &[&str]| {
        list.iter()
            .map(|word| render::json_string(word))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        r#"{{"exchanges":[{}],"segments":[{}]}}"#,
        words(&EXCHANGES.map(Exchange::as_str)),
        words(&SEGMENTS.map(Segment::as_str))
    )
}

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
    /// How many census keys this node sums: one per key, counted when the
    /// key is first seen. Exact along one read (a key is never removed from
    /// a census read forward; anything else rebuilds the index, `advance`).
    /// What a rows answer is capped by, read in one probe (WP-56b).
    keys: u64,
    /// The earliest and the latest month any key under this node names: the
    /// months a rows answer probes for this node, and no other (WP-56b).
    span: Option<(YearMonth, YearMonth)>,
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
        if old.is_none() {
            self.keys = self.keys.saturating_add(1);
        }
        let month = entry.key.month;
        self.span = Some(self.span.map_or((month, month), |(from, to)| {
            (from.min(month), to.max(month))
        }));
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
    /// Every key of the segment summed: the root answer's row for it, kept
    /// by the same `apply` as the nodes, so the root reads one node per
    /// segment and never walks the underlyings (WP-56b).
    total: Agg,
    /// Per (month, rung): the most rows any one key of the segment holds
    /// there, and how many keys the segment holds there. What a page needs
    /// beside one instrument's rows to say whether a short month is that
    /// instrument's hole or the whole store's (`denominators`, the rows
    /// answer below). Exact under `apply`: a key's rows only grow, so the
    /// maximum never has to come down, and a key is counted once, when it is
    /// first seen.
    denoms: BTreeMap<(YearMonth, Timeframe), (u64, u64)>,
}

impl Index {
    /// Every underlying, in exchange and symbol order.
    pub(crate) fn underlyings(&self) -> impl Iterator<Item = (&(Exchange, Symbol), &Underlying)> {
        self.underlyings.iter()
    }

    /// The generation of the census part these sums equal. Read by
    /// `/chain.json` (WP-32, D-3329), which names the census its chain was
    /// counted from.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether `symbol` is an underlying of this segment on `exchange`: one
    /// map probe.
    pub(crate) fn holds(&self, exchange: Exchange, symbol: Symbol) -> bool {
        self.underlyings.contains_key(&(exchange, symbol))
    }

    /// One expiry's option contracts, each as (strike paisa, `true` for a
    /// call, the earliest stamp the census holds for it in microseconds), in
    /// strike order, or `None` when the census holds no such expiry. Two
    /// probes and the expiry's own contracts, never a walk of the census:
    /// `/chain.json`'s point-in-time chain (WP-32, D-3329).
    pub(crate) fn expiry_options(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        expiry: Day,
    ) -> Option<Vec<(i64, bool, Option<i64>)>> {
        let chain = self
            .underlyings
            .get(&(exchange, symbol))?
            .expiries
            .get(&expiry)?;
        Some(
            chain
                .contracts
                .iter()
                .filter_map(|(id, agg)| match *id {
                    ContractId::Option { strike, call } => Some((strike, call, agg.first)),
                    ContractId::Future => None,
                })
                .collect(),
        )
    }

    /// One key moved from `old` to `new`.
    fn apply(&mut self, old: Option<&Held>, new: &Held) {
        let key = new.entry.key;
        self.total.apply(old, new);
        let denom = self
            .denoms
            .entry((key.month, key.timeframe))
            .or_insert((0, 0));
        denom.0 = denom.0.max(new.entry.rows);
        if old.is_none() {
            denom.1 = denom.1.saturating_add(1);
        }
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
    /// `rows=census-tuples-v1`: the census rows of the selection themselves
    /// ([`rows_answer`]), not its lists.
    rows: bool,
}

/// Reads the query, or names the parameter it cannot read.
fn asked(query: &str) -> Result<Asked, (axum::http::StatusCode, String)> {
    // NOTHING A CALLER SENT IS REPEATED WHOLE (WP-56b, the class of the
    // `since` echo): every word of the query is read through `word`, which
    // keeps the value for parsing and a capped copy for the sentence, and
    // the sentence itself is capped, whatever a parser's own error repeats.
    let bad = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            error_json("bad_request", &capped(&why, WHY_CHARS)),
        )
    };
    let echo = super::store_route::since_echo;
    let feed_word = param(query, "feed");
    let Some(feed) = (!feed_word.is_empty())
        .then(|| ingest::parse_vendor(&feed_word))
        .flatten()
    else {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            no_such_feed_json(&echo(&feed_word)),
        ));
    };
    let segment_word = param(query, "segment");
    let segment = Segment::parse(&segment_word).map_err(|why| {
        bad(format!(
            "segment {:?}: {}; one of {} is required",
            echo(&segment_word),
            capped(&why.to_string(), super::store_route::SINCE_ECHO_CHARS),
            SEGMENTS.map(Segment::as_str).join(", ")
        ))
    })?;
    let underlying = match param(query, "underlying").as_str() {
        "" => None,
        word => Some(Symbol::new(word).map_err(|why| {
            bad(format!(
                "underlying {:?}: {}",
                echo(word),
                capped(&why.to_string(), super::store_route::SINCE_ECHO_CHARS)
            ))
        })?),
    };
    let exchange = match param(query, "exchange").as_str() {
        "" => None,
        word => Some(Exchange::parse(word).map_err(|why| {
            bad(format!(
                "exchange {:?}: {}",
                echo(word),
                capped(&why.to_string(), super::store_route::SINCE_ECHO_CHARS)
            ))
        })?),
    };
    let expiry = match param(query, "expiry").as_str() {
        "" => None,
        word => {
            let (expiry, _) = contract_of(&format!("{word}-FUT"))
                .ok_or_else(|| bad(format!("expiry {:?} is not a YYYY-MM-DD day", echo(word))))?;
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
                .map_err(|_| bad(format!("since {:?} is not a generation", echo(word))))?,
        ),
    };
    let rows = match param(query, "rows").as_str() {
        "" => false,
        word if word == super::store_wire::ENCODING => {
            if underlying.is_none() {
                return Err(bad(
                    "rows needs an underlying: the census rows of one instrument".to_owned(),
                ));
            }
            if since.is_some() {
                return Err(bad(
                    "rows takes no since: it is one instrument's rows as the census holds \
                     them now; a delta of the census is /store.json?since="
                        .to_owned(),
                ));
            }
            true
        }
        word => {
            return Err(bad(format!(
                "rows {:?} is not an encoding this route writes; the one it writes is {}",
                super::store_route::since_echo(word),
                super::store_wire::ENCODING
            )));
        }
    };
    Ok(Asked {
        feed,
        segment,
        underlying,
        exchange,
        expiry,
        since,
        rows,
    })
}

/// The most characters of one refusal sentence: what a caller sent is
/// repeated capped ([`super::store_route::since_echo`]) and so is whatever a
/// parser's own error says about it.
const WHY_CHARS: usize = 640;

/// `text`, or its first `most` characters and how many were left out.
fn capped(text: &str, most: usize) -> String {
    let total = text.chars().count();
    if total <= most {
        return text.to_owned();
    }
    let kept: String = text.chars().take(most).collect();
    format!("{kept}...(+{} more)", total - most)
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
/// The feed of a ROOT request: `feed=` and nothing that narrows it. `None`
/// for every other query, which [`asked`] reads (and refuses by name).
fn root_feed(query: &str) -> Option<Vendor> {
    let feed = param(query, "feed");
    // An empty feed word is not a feed (`parse_vendor` reads it as its
    // default): it goes to `asked`, which refuses it by name.
    (!feed.is_empty()
        && [
            "segment",
            "underlying",
            "exchange",
            "expiry",
            "since",
            "rows",
        ]
        .iter()
        .all(|name| param(query, name).is_empty()))
    .then(|| ingest::parse_vendor(&feed))
    .flatten()
}

/// **THE ROOT OF THE SELECTION TREE** (WP-56b; DB-060, DB-G70):
/// `GET /census/facets.json?feed=<feed>` answers the segments the feed holds
/// a key in, in the census's order, each with its part's generation, its
/// underlyings counted, its rows per rung and its span, and the order the
/// census lists exchanges and segments in.
///
/// A page reads this ONE answer to draw the feed's first rung and to know
/// which segment requests exist, instead of the whole census (33 MB on GDFL
/// on 11 Oct 2026) or a list of segments written into the client. Cost: one
/// node read per segment ([`Index::total`]), whatever the census holds. A
/// segment the feed holds no key in is not listed.
fn root_answer(
    site: &Site,
    feed: Vendor,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let (_stamps, now) = vendor_census(site, feed);
    let census = now.census();
    let behind = super::store_route::behind(site, feed);
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
    let degraded = census.and_then(census::VendorCensus::degraded);
    let slot = now.slot();
    let mut out = format!(
        r#"{{"schema":1,"feed":{},"segment":null,"state":{},"degraded":{},"order":{},"segments":["#,
        render::json_string(feed.as_str()),
        render::json_string(state),
        degraded
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        order_json(),
    );
    let mut listed = 0_usize;
    for segment in SEGMENTS {
        let row = index(site, feed, segment, slot, |index| {
            (!index.underlyings.is_empty()).then(|| {
                let mut row = format!(
                    r#"{{"segment":{},"generation":{},"underlyings":{},"#,
                    render::json_string(segment.as_str()),
                    index.generation,
                    index.underlyings.len(),
                );
                agg_json(&mut row, &index.total, false);
                row.push('}');
                row
            })
        });
        if let Some(row) = row {
            if listed > 0 {
                out.push(',');
            }
            out.push_str(&row);
            listed += 1;
        }
    }
    out.push_str("]}");
    (axum::http::StatusCode::OK, headers, out)
}

pub(super) fn answer(
    site: &Site,
    query: &str,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    if let Some(feed) = root_feed(query) {
        return root_answer(site, feed);
    }
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
    let (body, generation) = if asked.rows {
        rows_answer(site, &asked, census, slot, state, degraded.as_deref())
    } else {
        index(site, asked.feed, asked.segment, slot, |index| {
            (
                render_answer(index, &asked, state, degraded.as_deref()),
                index.generation,
            )
        })
    };
    // THE GENERATION A `since` IS SPELLED IN, in a header as well as the
    // body, so a client never keys a delta on the vendor-wide sum (WP-56;
    // DB-B56): this part's own, the body's `generation`.
    let mut headers = headers;
    headers.insert(
        axum::http::HeaderName::from_static(super::store_route::CENSUS_PART_GENERATION_HEADER),
        axum::http::HeaderValue::from(generation),
    );
    match body {
        Ok(body) => (axum::http::StatusCode::OK, headers, body),
        Err((code, body)) => (code, headers, body),
    }
}

/// The most census rows one rows answer carries. Past it the answer is a
/// refusal that names the count of rows the selection HOLDS (read off the
/// index's node in one probe), never a body sized by the store (D-3327;
/// WP-56b).
pub(crate) const ROWS_CAP: usize = 4_096;

/// The most keys one rows answer probes in the census to find its rows: a
/// budget of its own, four probes for each row the cap allows. A selection
/// that holds no more than [`ROWS_CAP`] rows is refused under this budget
/// only when its contracts are so sparse over their own months that naming
/// them would take more probes than this, and the refusal says so.
pub(crate) const PROBE_CAP: usize = ROWS_CAP * 4;

/// A refusal of a rows answer: the status and the body.
type RowsRefusal = (axum::http::StatusCode, String);

/// What a `rows_too_many` refusal counted when it refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Counted {
    /// The census rows the selection holds, exactly: the node's own count.
    Rows(u64),
    /// The selection holds `held` rows, within the cap, and naming them
    /// would take more than [`PROBE_CAP`] probes of the census.
    Probes { held: u64 },
}

/// The keys the census may hold for the selection `asked` names, read off
/// the index's own nodes (no walk of the census), and the rows per rung
/// those nodes say they sum to. `None` for the exchange when the feed holds
/// no such underlying in the segment.
///
/// THE CAP COUNTS CENSUS ROWS, NOT CANDIDATES: every node counts the keys it
/// sums (`Agg::keys`), so whether the selection holds more than [`ROWS_CAP`]
/// rows is one probe, and the refusal states that count. (Until the second
/// check of WP-56b the cap counted contracts x months of the chain x rungs,
/// and refused NIFTY 2020-06-25, which holds 1,040 rows, as 'more than
/// 4,096'.)
///
/// INDEX and CASH: one key per (month, rung) of the underlying, each held.
/// FNO: per contract, one candidate per (month of the expiry inside the
/// contract's own first..last month, rung of the contract); a candidate the
/// census does not hold is dropped by the probe that follows, and the
/// candidates are bounded by [`PROBE_CAP`].
fn rows_keys(
    index: &Index,
    asked: &Asked,
) -> Result<(Option<Exchange>, Vec<EntryKey>, Rows), RowsRefusal> {
    let bad = axum::http::StatusCode::BAD_REQUEST;
    let Some(symbol) = asked.underlying else {
        return Ok((None, Vec::new(), Rows::new()));
    };
    let chosen: Vec<(&(Exchange, Symbol), &Underlying)> = EXCHANGES
        .into_iter()
        .filter(|exchange| asked.exchange.is_none_or(|wanted| wanted == *exchange))
        .filter_map(|exchange| index.underlyings.get_key_value(&(exchange, symbol)))
        .collect();
    if chosen.len() > 1 {
        let exchanges: Vec<&str> = chosen
            .iter()
            .map(|((exchange, _), _)| exchange.as_str())
            .collect();
        return Err((
            bad,
            error_json(
                "ambiguous_underlying",
                &format!(
                    "{} is held on {}; name one with exchange=",
                    symbol.as_str(),
                    exchanges.join(", ")
                ),
            ),
        ));
    }
    let Some(((exchange, _), under)) = chosen.first().copied() else {
        return Ok((None, Vec::new(), Rows::new()));
    };
    let key = |contract, month, timeframe| EntryKey {
        contract,
        exchange: *exchange,
        segment: asked.segment,
        symbol,
        timeframe,
        month,
    };
    if asked.segment == Segment::Fno {
        let (keys, sums) = chain_keys(under, asked, symbol, &key)?;
        return Ok((Some(*exchange), keys, sums));
    }
    if past_cap(under.agg.keys) {
        return Err(rows_too_many(asked, symbol, Counted::Rows(under.agg.keys)));
    }
    let mut keys: Vec<EntryKey> = Vec::new();
    for (month, agg) in &under.months {
        for timeframe in agg.rows.keys() {
            keys.push(key(None, *month, *timeframe));
        }
    }
    Ok((Some(*exchange), keys, under.agg.rows.clone()))
}

/// Whether a node that sums `held` keys holds more than [`ROWS_CAP`] rows.
fn past_cap(held: u64) -> bool {
    usize::try_from(held).map_or(true, |held| held > ROWS_CAP)
}

/// The refusal of a selection the cap or the probe budget refuses: what was
/// counted and the count, the cap, and what to ask instead.
fn rows_too_many(asked: &Asked, symbol: Symbol, counted: Counted) -> RowsRefusal {
    let at = if asked.expiry.is_some() {
        " at this expiry"
    } else {
        ""
    };
    let next = if asked.expiry.is_some() {
        "read the census whole from /store.json"
    } else {
        "narrow it with expiry=, or read the census whole from /store.json"
    };
    let (reached, word, why) = match counted {
        Counted::Rows(held) => (
            held,
            "census rows",
            format!(
                "{} in {} holds {held} census rows{at}, more than the {ROWS_CAP} one rows \
                 answer carries: {next}",
                symbol.as_str(),
                asked.segment.as_str(),
            ),
        ),
        Counted::Probes { held } => (
            held,
            "probes",
            format!(
                "{} in {} holds {held} census rows{at}, within the {ROWS_CAP} one rows answer \
                 carries, but its contracts are spread so thinly over their months that \
                 naming them takes more than {PROBE_CAP} probes of the census: {next}",
                symbol.as_str(),
                asked.segment.as_str(),
            ),
        ),
    };
    (
        axum::http::StatusCode::BAD_REQUEST,
        format!(
            r#"{{"error":"rows_too_many","why":{},"cap":{ROWS_CAP},"reached":{reached},"counted":{},"probe_cap":{PROBE_CAP}}}"#,
            render::json_string(&why),
            render::json_string(word),
        ),
    )
}

/// [`rows_keys`] for a segment whose keys name contracts: the candidate keys
/// of the expiry asked (or of every expiry), and the rows per rung those
/// expiries' nodes sum to.
fn chain_keys(
    under: &Underlying,
    asked: &Asked,
    symbol: Symbol,
    key: &dyn Fn(Option<brutex_core::instrument::Contract>, YearMonth, Timeframe) -> EntryKey,
) -> Result<(Vec<EntryKey>, Rows), RowsRefusal> {
    // A KEY THIS INDEX CANNOT NAME IS NOT LEFT OUT IN SILENCE: its contract
    // text is not the store's spelling, so no candidate would find it.
    if under.unread_contracts > 0 {
        return Err((
            axum::http::StatusCode::CONFLICT,
            error_json(
                "rows_unreadable",
                &format!(
                    "{} key(s) of {} carry a contract text this index cannot read, so its \
                     rows cannot be listed from the index; read the census whole from \
                     /store.json",
                    under.unread_contracts,
                    symbol.as_str()
                ),
            ),
        ));
    }
    // THE CAP FIRST, IN ONE PROBE, BEFORE ANYTHING IS COLLECTED: the rows the
    // selection holds are the node's own count. Past it nothing below runs,
    // so the walk that follows is over at most ROWS_CAP contracts (each
    // holds a key) in at most ROWS_CAP expiries.
    let one = asked
        .expiry
        .and_then(|day| under.expiries.get_key_value(&day));
    let held = match asked.expiry {
        Some(_) => one.map_or(0, |(_, chain)| chain.agg.keys),
        None => under.agg.keys,
    };
    if past_cap(held) {
        return Err(rows_too_many(asked, symbol, Counted::Rows(held)));
    }
    let mut only = one.into_iter();
    let mut every = under.expiries.iter();
    let chains: &mut dyn Iterator<Item = (&Day, &Expiry)> = if asked.expiry.is_some() {
        &mut only
    } else {
        &mut every
    };
    let mut keys: Vec<EntryKey> = Vec::new();
    let mut sums = Rows::new();
    for (expiry, chain) in chains {
        for (timeframe, rows) in &chain.agg.rows {
            let sum = sums.entry(*timeframe).or_insert(0);
            *sum = sum.saturating_add(*rows);
        }
        for (id, agg) in &chain.contracts {
            let text = match *id {
                ContractId::Future => format!("{expiry}-FUT"),
                ContractId::Option { strike, call } => {
                    format!("{expiry}-{strike}-{}", if call { "CE" } else { "PE" })
                }
            };
            // The index read this id FROM a contract of the store's own
            // spelling, so the same text parses; one that did not would be
            // a key left out, and is refused by name instead.
            let Some(contract) = brutex_core::instrument::Contract::parse(&text) else {
                return Err((
                    axum::http::StatusCode::CONFLICT,
                    error_json(
                        "rows_unreadable",
                        &format!("contract {text:?} of the index does not parse back"),
                    ),
                ));
            };
            // THE CONTRACT'S OWN MONTHS, not the chain's: a contract listed
            // for three of the chain's twenty-two months is probed in those
            // three (and in any month between its first and last it did not
            // trade in, which the probe drops).
            let Some((from, to)) = agg.span else {
                continue;
            };
            for month in chain.months.range(from..=to).map(|(month, _)| *month) {
                for timeframe in agg.rows.keys() {
                    if keys.len() >= PROBE_CAP {
                        return Err(rows_too_many(asked, symbol, Counted::Probes { held }));
                    }
                    keys.push(key(Some(contract), month, *timeframe));
                }
            }
        }
    }
    Ok((keys, sums))
}

/// **ONE INSTRUMENT'S CENSUS ROWS** (WP-56b; DB-060, DB-G59, DB-G70, DB-A31):
/// `GET /census/facets.json?feed=&segment=&underlying=[&expiry=]&rows=census-tuples-v1`
/// answers the census rows of that selection in the SAME wire `/store.json`
/// writes (`census`, read by the same decoder), with what a page needs
/// beside them: the fullest row and the key count of the whole feed at each
/// (month, rung) the rows name (`denominators`), the rows counted (`held`),
/// and the part generation they were read at.
///
/// A page draws its first paint from this answer (11,822 B of lists and
/// about as much of rows for GDFL NIFTY spot) instead of the whole census
/// (33,349,105 B on 11 Oct 2026).
///
/// COST: the cap is decided in one probe of the index (the node's own count
/// of the rows it holds); the keys are then read off the index's nodes
/// ([`rows_keys`]), each one probe of the census, at most [`PROBE_CAP`] of
/// them: bounded by the two caps, whatever the store holds. The denominators
/// are one probe per (segment, month, rung) named.
///
/// IT EQUALS THE CENSUS OR IT IS REFUSED: the rows found must sum, per rung,
/// to what the index's node says the selection holds; a difference (a key
/// the candidates missed) is `rows_incomplete`, never a short list.
fn rows_answer(
    site: &Site,
    asked: &Asked,
    census: Option<&census::VendorCensus>,
    slot: &Arc<Slot>,
    state: &str,
    degraded: Option<&str>,
) -> (Result<String, RowsRefusal>, u64) {
    let (found, generation) = index(site, asked.feed, asked.segment, slot, |index| {
        (rows_keys(index, asked), index.generation)
    });
    let (exchange, keys, sums) = match found {
        Ok(found) => found,
        Err(refusal) => return (Err(refusal), generation),
    };
    let part = part_of(slot, asked.segment);
    let mut entries: Vec<(census::Series, YearMonth)> = Vec::with_capacity(keys.len());
    let mut counted = Rows::new();
    let mut bars = 0_u64;
    for key in &keys {
        let Some(held) = part.and_then(|part| part.held(key)) else {
            continue;
        };
        let sum = counted.entry(key.timeframe).or_insert(0);
        *sum = sum.saturating_add(held.entry.rows);
        bars = bars.saturating_add(held.entry.rows);
        entries.push((census::Series::of(key), key.month));
    }
    if counted != sums {
        return (
            Err((
                axum::http::StatusCode::CONFLICT,
                error_json(
                    "rows_incomplete",
                    "the rows this index can name do not sum to what the census holds for \
                     the selection (a key outside the index's reading); read the census \
                     whole from /store.json",
                ),
            )),
            generation,
        );
    }
    // THE WHOLE BODY'S ORDER (`census::held_entries`): newest month first,
    // then the series' own, so these rows read like a slice of /store.json.
    entries.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let denoms = feed_denominators(site, asked.feed, slot, &entries);
    let rows = match census {
        Some(census) => {
            super::store_wire::compact(std::slice::from_ref(census), &entries, asked.feed)
        }
        None => super::store_wire::compact(&[], &entries, asked.feed),
    };
    let rows = match rows {
        Ok(rows) => rows,
        Err(why) => {
            return (
                Err((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    error_json("census_encoding_failed", &why),
                )),
                generation,
            );
        }
    };
    let held = (entries.len(), bars);
    (
        Ok(rows_body(
            asked, exchange, generation, state, degraded, held, &denoms, &rows,
        )),
        generation,
    )
}

/// Per (month, rung) named by `entries`: the fullest row and the key count
/// of the WHOLE feed there, summed over every segment's index. One probe
/// per (segment, month, rung).
fn feed_denominators(
    site: &Site,
    feed: Vendor,
    slot: &Arc<Slot>,
    entries: &[(census::Series, YearMonth)],
) -> BTreeMap<(YearMonth, Timeframe), (u64, u64)> {
    let wanted: BTreeSet<(YearMonth, Timeframe)> = entries
        .iter()
        .map(|(series, month)| (*month, series.timeframe))
        .collect();
    let mut denoms: BTreeMap<(YearMonth, Timeframe), (u64, u64)> = BTreeMap::new();
    for segment in SEGMENTS {
        index(site, feed, segment, slot, |index| {
            for at in &wanted {
                if let Some((fullest, support)) = index.denoms.get(at) {
                    let sum = denoms.entry(*at).or_insert((0, 0));
                    sum.0 = sum.0.max(*fullest);
                    sum.1 = sum.1.saturating_add(*support);
                }
            }
        });
    }
    denoms
}

/// The rows answer's body: what was asked, the part generation, the rows
/// counted (`held`: census rows, then bars), the denominators, and the
/// tuples (`census`, already encoded).
#[expect(
    clippy::too_many_arguments,
    reason = "one body written from the eight things it states; a struct would be built \
              once, here, to be taken apart on the next line"
)]
fn rows_body(
    asked: &Asked,
    exchange: Option<Exchange>,
    generation: u64,
    state: &str,
    degraded: Option<&str>,
    (months, bars): (usize, u64),
    denoms: &BTreeMap<(YearMonth, Timeframe), (u64, u64)>,
    census_rows: &str,
) -> String {
    let word =
        |text: Option<String>| text.map_or_else(|| "null".to_owned(), |t| render::json_string(&t));
    let mut out = format!(
        r#"{{"schema":1,"feed":{},"segment":{},"underlying":{},"exchange":{},"expiry":{},"generation":{generation},"state":{},"degraded":{},"order":{},"cap":{ROWS_CAP},"held":{{"months":{months},"rows":{bars}}},"denominators":["#,
        render::json_string(asked.feed.as_str()),
        render::json_string(asked.segment.as_str()),
        word(asked.underlying.map(|symbol| symbol.as_str().to_owned())),
        word(exchange.map(|exchange| exchange.as_str().to_owned())),
        word(asked.expiry.map(|day| day.to_string())),
        render::json_string(state),
        word(degraded.map(str::to_owned)),
        order_json(),
    );
    for (at, ((month, timeframe), (fullest, support))) in denoms.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "[{},{},{fullest},{support}]",
            render::json_string(&month.to_string()),
            render::json_string(timeframe.as_str())
        );
    }
    out.push_str(r#"],"census":"#);
    out.push_str(census_rows);
    out.push('}');
    out
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
            // A GENERATION THIS PART HAS NOT REACHED is not "too old": it is a
            // value this part never published (the vendor-wide sum, another
            // part's, a typo). Refused as a conflict naming the generation to
            // use, so a client does not reload in a loop believing it fell
            // behind (WP-56; DB-B56).
            if since > index.generation {
                return Err((
                    axum::http::StatusCode::CONFLICT,
                    format!(
                        r#"{{"error":"since_in_future","why":{},"since":{since},"generation":{}}}"#,
                        render::json_string(
                            "that generation is past this part's newest; a delta is asked \
                             since the body's generation (x-brutex-census-part-generation), \
                             never since the vendor-wide x-brutex-census-generation"
                        ),
                        index.generation
                    ),
                ));
            }
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
        Some(symbol) => EXCHANGES
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
        r#"{{"schema":1,"feed":{},"segment":{},"underlying":{},"expiry":{},"generation":{},"since":{},"state":{},"degraded":{},"order":{},"underlyings":["#,
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
        order_json(),
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
#[path = "facets_tests.rs"]
mod tests;
