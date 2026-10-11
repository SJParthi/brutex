//! `/chain.json` -- one underlying's option chain for one expiry at an
//! instant, counted over the strikes STORED BY THEN, with the spot, the local
//! step, the ATM strike and each strike's rung (D-3329).
//!
//! ```text
//! GET /chain.json?feed=gdfl&underlying=NIFTY&expiry=2019-03-28&timeframe=1s&at=1537347495
//!     [&exchange=NSE][&asof=prev][&strike=1150000&side=CE]
//! ```
//!
//! # The defect this closes
//!
//! /db counted a moneyness rung along every strike the WHOLE census ever held
//! for an expiry, from any month and even from other underlyings (DB-008,
//! DB-037, DB-G04, DB-G62, DB-G73): the NIFTY 2019-03-28 11,500 CE at
//! 2018-09-19 14:28:15 read OTM+5 over 83 strikes, 75 of them first stored
//! from 2018-10 on; a 2018 bar's rung moved as the import reached 2020; a
//! frozen census truncated the ladder; and a three-strike long-dated chain
//! called a strike 754.70 from spot "ATM".
//!
//! # The basis: stored by then
//!
//! A strike is in the chain at `t` when the first stored bar of either side
//! (the census's earliest stamp for that contract, at any rung) is at or
//! before `t`, and that side's key was not renamed away on or before `t`'s IST
//! day (D-3331). Exchange listing dates are not sourced (`docs/00-charter.md`
//! records none), so "listed by then" is NOT claimed: `basis` is
//! `stored_by_then`. That is a statement about this store, and it is the only
//! one a reader can check. No strike first stored after `t` is counted, named
//! or hinted at: its stamp is a later fact (`CLAUDE.md` §3 rule 7).
//!
//! **Renamed away** is read from the import's lineage ledger (D-3331): its
//! `lineage` and `stitched` records date each corporate-action adjustment of
//! an underlying (its ex-date). D-3331 records that at a share's bonus or
//! split the vendor renames EACH strike to its adjusted value, and the
//! detector links only the contracts that traded across the ex-date, so the
//! ledger's records are the evidence that an adjustment happened, not the
//! list of every key it renamed. From an ex-date `E` of the underlying, a side
//! whose key was first stored before `E` (or that a `lineage` record names as
//! the OLD key at `E`) is out of the chain, unless a record says the key holds
//! a contract from `E` on:
//!
//! * a `lineage` record's NEW key, or a `stitched` key whose `after` names the
//!   pre-ex contract renamed onto it, holds that contract from `E`'s first
//!   instant (the contract existed before, under another name);
//! * a `stitched` key whose `after` is `-` holds a contract NEWLY LISTED on it
//!   on some day from `E`, which the ledger does not date. It is back in the
//!   chain from its first stored bar at or after `E`'s first instant, read
//!   from the key's own month files (`E`'s month through the expiry's): the
//!   stored-by-then rule applied to the contract the key now holds. NTPC's
//!   130, 132.50, 135, 140 and 155 keys were relisted on 2019-03-20, the day
//!   after the ex-date, and are not counted on the ex-date itself.
//!
//! Without that rule a post-ex bar was ranked against the pre-ex keys nobody
//! traded on the ex-date (DB-G73: NTPC 2019-03-28's 2.50 grid beside its
//! adjusted strikes). A record counts only from its ex-date, on or before
//! `t`'s IST day, and a stamp after `t` only ever keeps a key out, exactly as
//! a first stored stamp after `t` does, so nothing later than the bar is
//! counted or named.
//!
//! # The rung
//!
//! The spot is the underlying's close at `t` on the same feed and rung,
//! through `/spot.json`'s own reader (exact by default, `asof=prev` opted in,
//! D-3328). ATM is the stored strike nearest spot, the lower of two
//! equidistant ones. `k` is a strike's offset from ATM along the chain; a
//! call is `ITM-|k|` below ATM and `OTM+k` above it, a put the reverse (the
//! convention `web/src/lib/db/chain.js` names). `step_local` is the smaller
//! gap from the ATM strike to its neighbours.
//!
//! # Refusals, by name and in this order
//!
//! * `census_absent` -- the feed has no census to count strikes from.
//! * `census_state_unknown` -- the import journal that says whether the
//!   census is behind would not read.
//! * `census_degraded` -- the census loaded only by stepping over damage
//!   (`census::VendorCensus::degraded`): it is a recovered, EARLIER generation,
//!   so strikes a later generation had counted may be missing from it, and a
//!   ladder counted over it could be truncated anywhere. Refused, with the
//!   reason it stepped over, rather than ranked on a census that says it is
//!   short (`CLAUDE.md` §4; D-3327 names the state, D-3329 the refusal shape).
//! * `census_behind` -- the census is `behind` (D-3327) from day `F`, and
//!   both `t`'s IST day and the expiry are on or after `F`: strikes first
//!   stored from `F` are not counted yet, so a ladder at `t` would be
//!   truncated (DB-G04). An instant before `F` is not refused: every strike
//!   stored before it was counted. (D-3329 states the expiry half; the
//!   instant half is what makes the verification's 2019 bars of a 2023 expiry
//!   answerable while the census is behind from 2019-11-14.)
//! * `expiry_not_in_census` -- the census holds no option of this underlying
//!   and expiry.
//! * the spot's own word (`no_spot_bar`, `no_spot_in_session`,
//!   `no_spot_file`, `spot_unreadable`, `spot_order_broken`) -- no spot, no
//!   money to count from.
//! * `no_strike_stored_by_then` -- the chain is empty at `t`.
//! * `ladder_too_sparse` -- one strike only, or the ATM strike is more than
//!   `step_local` from spot: the nearest stored strike is not the money
//!   (DB-G62: the 2023-12-28 chain of [9000, 12700, 13000] at spot 11,945.30).
//!
//! A refused instant still carries the strikes stored by then (in the full
//! form) and its spot when one was read; it carries no ATM and no rung.
//!
//! # Two forms
//!
//! Without `strike`, each instant lists every strike stored by then, with
//! `k`, both rungs and each side's `*_first`: the stamp that side is counted
//! from (its first stored bar, or after an adjustment the stamp its key holds
//! a contract from; `null` for a side not in the chain then): at most
//! [`MAX_FULL_CHAINS`] instants. With `strike` and `side`, each instant carries that one
//! contract's rung (`pick`) and the chain's size: at most [`MAX_CHAIN_TIMES`]
//! instants, so a page of bars is one request.
//!
//! # Cost, stated rather than claimed
//!
//! Per request: the per-vendor census read (`census_cache::vendor_census`),
//! the FNO facet index's probe of this one expiry (WP-30's index, moved
//! forward by appended entries, never a walk of the census), the lineage
//! ledger's stamp (read again only when it moves), one month file opened per
//! distinct month asked, and, for an expiry the ledger dates an adjustment
//! for, one lookup per newly listed key in at most the months from the
//! ex-date's to the expiry's. Per instant: one spot lookup (`/spot.json`'s
//! bound: O(1) through a `.tix`) and one pass over THIS EXPIRY's strikes,
//! O(S) with S the strikes the expiry holds (bounded by the expiry, not by
//! the store: 140 for a NIFTY monthly in 2019). The chain is built per
//! (underlying, expiry) at the census generation the answer names. It runs
//! on the blocking pool behind `crate::detail::run_store_read`'s admission
//! (DB-066). Measured timings are in WP-32's report, not restated here.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use brutex_core::instrument::{Contract, Exchange, Expiry, OptionSide, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::gdfl_lineage::{LineageRecord, StitchedRecord};
use store::file::BarFile;
use store::path::{IST_OFFSET_SECS, PathParts, Timeframe, YearMonth};

use super::optiongreeks::option_parts;
use super::spot::{
    Answered, AsOf, MAX_SPOT_TIMES, Spot, bad_request, ist_day, json_headers, month_of,
    not_admitted, spot_at, spot_fields, times_param,
};
use super::{Loaded, Site, ingest, param, render, timeframe_param, vendor_census};
use crate::bars::OpenRefusalKind;
use crate::census;

/// The most instants one request may ask for with `strike` and `side`: the
/// `/spot.json` bound, for the same 8 KiB request target.
pub(super) const MAX_CHAIN_TIMES: usize = MAX_SPOT_TIMES;

/// The most instants one request may ask for WITHOUT `strike`: each lists the
/// whole chain, so the body grows with strikes times instants and is held to
/// a few.
pub(super) const MAX_FULL_CHAINS: usize = 8;

/// The most bytes of the lineage ledger read. The ledger holds a few lines
/// per corporate action and per vendor re-dating; a file past this is not
/// one the import wrote, and is refused by name rather than read whole.
const MAX_LINEAGE_BYTES: u64 = 64 * 1024 * 1024;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// What the basis is, said on the wire. D-3329.
pub(super) const BASIS_NOTE: &str = "a strike is in the chain at t when the first stored bar of either side is at or before t and that side's key was not renamed away by t's IST day: from an ex-date the lineage ledger dates (D-3331), a key first stored before it is out unless a record says it holds a contract from then; exchange listing dates are not sourced, so 'listed by then' is not claimed (D-3329)";

/// Everything `/chain.json` reads off the query string.
#[derive(Debug)]
struct Asked {
    feed: Vendor,
    exchange: Option<Exchange>,
    underlying: Symbol,
    expiry: Expiry,
    /// The same day on the session calendar, which the census keys on.
    expiry_day: pull::session::Day,
    timeframe: Timeframe,
    asof: AsOf,
    pick: Option<(i64, OptionSide)>,
    times: Vec<i64>,
}

/// `YYYY-MM-DD` as an expiry.
fn expiry_param(word: &str) -> Option<Expiry> {
    let mut parts = word.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if (y.len(), m.len(), d.len()) != (4, 2, 2) {
        return None;
    }
    Expiry::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()
}

impl Asked {
    /// Reads one, or names the first thing wrong with it.
    fn parse(query: &str) -> Result<Self, String> {
        let feed_word = param(query, "feed");
        let Some(feed) = ingest::parse_vendor(&feed_word) else {
            return Err(format!("{feed_word:?} is not a feed this build can read"));
        };
        let word = param(query, "underlying");
        if word.is_empty() {
            return Err("underlying is required: the option's symbol as the store files it".into());
        }
        let underlying = Symbol::new(&word).map_err(|why| format!("underlying {word:?}: {why}"))?;
        let exchange = match param(query, "exchange").as_str() {
            "" => None,
            word => Some(Exchange::parse(word).map_err(|why| format!("exchange {word:?}: {why}"))?),
        };
        let word = param(query, "expiry");
        let expiry = expiry_param(&word)
            .ok_or_else(|| format!("expiry {word:?} is not a YYYY-MM-DD day"))?;
        let expiry_day = pull::session::Day::new(expiry.year(), expiry.month(), expiry.day())
            .map_err(|why| format!("expiry {word:?}: {why}"))?;
        if param(query, "timeframe").is_empty() {
            return Err(
                "timeframe is required: the spot is read at the option bar's own rung".into(),
            );
        }
        let timeframe = timeframe_param(query)?;
        let asof = AsOf::parse(query)?;
        let pick = match (
            param(query, "strike").as_str(),
            param(query, "side").as_str(),
        ) {
            ("", "") => None,
            (strike, side) => {
                let paisa = strike
                    .parse::<i64>()
                    .ok()
                    .filter(|paisa| *paisa > 0 && strike.bytes().all(|b| b.is_ascii_digit()))
                    .ok_or_else(|| {
                        format!("strike {strike:?} is not a strike in paisa; strike and side go together")
                    })?;
                let side = match side {
                    "CE" => OptionSide::Call,
                    "PE" => OptionSide::Put,
                    other => {
                        return Err(format!(
                            "side {other:?} is not CE or PE; strike and side go together"
                        ));
                    }
                };
                Some((paisa, side))
            }
        };
        let cap = if pick.is_some() {
            MAX_CHAIN_TIMES
        } else {
            MAX_FULL_CHAINS
        };
        let times = times_param(query, "at", cap)?;
        Ok(Self {
            feed,
            exchange,
            underlying,
            expiry,
            expiry_day,
            timeframe,
            asof,
            pick,
            times,
        })
    }
}

/// One strike of the expiry: its paisa and each side's first stored stamp
/// and the adjustments that renamed or reused its key.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Strike {
    paisa: i64,
    call: Side,
    put: Side,
}

/// One side of one strike.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Side {
    /// The earliest stamp the census holds for the contract, microseconds.
    first: Option<i64>,
    /// Each adjustment of the underlying that reached this key, ascending by
    /// ex-date: the ex-date's IST day number, and the stamp (microseconds)
    /// from which the key holds a contract again, `None` when it was renamed
    /// away and holds none. See the module header.
    adjusted: Vec<(i64, Option<i64>)>,
}

impl Side {
    /// Whether this side is in the chain at the instant `at` (IST day `day`):
    /// first stored by then, and held by the latest adjustment on or before
    /// that day, if any.
    fn stored_by(&self, at: i64, day: i64) -> bool {
        self.since(at, day).is_some()
    }

    /// The stamp this side is counted from at `at` (IST day `day`), when it
    /// is in the chain then: its first stored bar, or, after an adjustment
    /// that reached its key, the stamp the key holds a contract from. Never
    /// a stamp after `at`.
    fn since(&self, at: i64, day: i64) -> Option<i64> {
        let first = self.first.filter(|first| *first <= at)?;
        match self.adjusted.iter().rev().find(|(ex, _)| *ex <= day) {
            None => Some(first),
            Some((_, held)) => held.filter(|from| *from <= at),
        }
    }

    /// [`Self::since`] in seconds, for the wire.
    fn first_by(&self, at: i64) -> Option<i64> {
        self.since(at, ist_day(at))
            .map(|since| since.div_euclid(MICROS))
    }
}

/// The expiry's strikes from the facet index's contracts, in strike order.
fn strikes_of(options: &[(i64, bool, Option<i64>)]) -> Vec<Strike> {
    let mut by: BTreeMap<i64, Strike> = BTreeMap::new();
    for &(paisa, call, first) in options {
        let strike = by.entry(paisa).or_insert_with(|| Strike {
            paisa,
            call: Side::default(),
            put: Side::default(),
        });
        let side = if call {
            &mut strike.call
        } else {
            &mut strike.put
        };
        side.first = first;
    }
    by.into_values().collect()
}

/// The rung name for an offset of `k` stored strikes from the money.
fn rung(k: i64, call: bool) -> String {
    if k == 0 {
        "ATM".to_owned()
    } else if call == (k < 0) {
        format!("ITM-{}", k.unsigned_abs())
    } else {
        format!("OTM+{}", k.unsigned_abs())
    }
}

/// The chain at one instant: the strikes stored by then, the ATM index and
/// local step, or the refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ladder {
    /// Indexes into the expiry's strikes of those stored by then, ascending.
    stored: Vec<usize>,
    /// The position in `stored` of the ATM strike, when the money was found.
    atm: Option<usize>,
    /// The smaller gap from the ATM strike to its neighbours, paisa.
    step_local: Option<i64>,
    /// Why no ATM was named.
    refusal: Option<(&'static str, String)>,
}

/// The ladder at `at` over `strikes`, measured from `spot`, or refused with
/// the reason `spot` carries when there is no money to measure from (no
/// spot, or a census refusal already decided).
fn ladder(strikes: &[Strike], at: i64, spot: Result<i64, (&'static str, String)>) -> Ladder {
    let day = ist_day(at);
    let stored: Vec<usize> = strikes
        .iter()
        .enumerate()
        .filter(|(_, strike)| strike.call.stored_by(at, day) || strike.put.stored_by(at, day))
        .map(|(n, _)| n)
        .collect();
    let refused = |stored: Vec<usize>, step_local, why: &'static str, detail: String| Ladder {
        stored,
        atm: None,
        step_local,
        refusal: Some((why, detail)),
    };
    let spot = match spot {
        Ok(spot) => spot,
        Err((why, detail)) => return refused(stored, None, why, detail),
    };
    let paisa = |at: usize| {
        stored
            .get(at)
            .and_then(|&n| strikes.get(n))
            .map_or(0, |strike| strike.paisa)
    };
    if stored.is_empty() {
        return refused(
            stored,
            None,
            "no_strike_stored_by_then",
            "no strike of this expiry has a bar stored at or before this instant".to_owned(),
        );
    }
    // THE NEAREST STORED STRIKE, THE LOWER OF TWO EQUIDISTANT ONES.
    let mut money = 0usize;
    for at in 1..stored.len() {
        if paisa(at).abs_diff(spot) < paisa(money).abs_diff(spot) {
            money = at;
        }
    }
    let below = money
        .checked_sub(1)
        .map(|before| paisa(money) - paisa(before));
    let above = (money + 1 < stored.len()).then(|| paisa(money + 1) - paisa(money));
    let step_local = match (below, above) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (one, other) => one.or(other),
    };
    let Some(step) = step_local else {
        return refused(
            stored,
            None,
            "ladder_too_sparse",
            "one strike is stored by this instant, and a rung is a count of strikes from the money"
                .to_owned(),
        );
    };
    let off = paisa(money).abs_diff(spot);
    if off > step.unsigned_abs() {
        let nearest = paisa(money);
        return refused(
            stored,
            Some(step),
            "ladder_too_sparse",
            format!(
                "the nearest strike stored by this instant, {nearest} paisa, is {off} paisa from spot \
                 {spot}, more than the local step of {step} paisa, so it is not the money"
            ),
        );
    }
    Ladder {
        stored,
        atm: Some(money),
        step_local: Some(step),
        refusal: None,
    }
}

/// What the lineage ledger said, as this request found it.
#[derive(Debug, Clone)]
enum Lineage {
    /// The feed keeps no ledger (only the GDFL import writes one, D-3331).
    NotKept,
    /// The ledger is not on disk: no rename has been recorded.
    Absent,
    /// Read.
    Read(Arc<LineageIndex>),
    /// It would not read; the reason, verbatim.
    Unreadable(String),
}

/// (underlying, expiry) to the (strike paisa, `true` for a call, ex-date IST
/// day number) of each key a ledger record names.
type Marks = HashMap<(Symbol, Expiry), HashSet<(i64, bool, i64)>>;

/// The ledger's `lineage` and `stitched` records, by underlying and expiry,
/// and each underlying's ex-dates.
#[derive(Debug, Default)]
struct LineageIndex {
    /// Each key a `lineage` record names as the OLD key at its ex-date.
    renamed: Marks,
    /// Each key that holds a renamed contract FROM an ex-date: a `lineage`
    /// record's NEW key, or a `stitched` key whose `after` names one.
    reused: Marks,
    /// Each `stitched` key whose `after` is `-`: a contract newly listed on
    /// it from some day on or after the ex-date.
    listed: Marks,
    /// Each underlying's ex-dates, as IST day numbers, ascending and once
    /// each: the adjustments the records date.
    adjusted: HashMap<Symbol, Vec<i64>>,
    /// Records read.
    records: usize,
    /// Lines that are neither an `alias`, a `lineage` nor a `stitched`
    /// record this build reads: counted and said, never skipped silently.
    unread: usize,
}

impl LineageIndex {
    /// The ledger's text, read.
    fn of(text: &str) -> Self {
        let mut index = Self::default();
        for line in text.lines().filter(|line| !line.is_empty()) {
            let read = if let Some(record) = LineageRecord::parse(line) {
                // Both halves or neither: a half kept is a rename without its
                // successor, or the reverse.
                match (key_of(record.old), key_of(record.new)) {
                    (Some(old), Some(new)) => {
                        index.put(Map::Renamed, record.underlying, old, record.ex_date);
                        index.put(Map::Reused, record.underlying, new, record.ex_date);
                        true
                    }
                    _ => false,
                }
            } else if let Some(record) = StitchedRecord::parse(line) {
                let map = if record.after.is_some() {
                    Map::Reused
                } else {
                    Map::Listed
                };
                key_of(record.key).is_some_and(|key| {
                    index.put(map, record.underlying, key, record.ex_date);
                    true
                })
            } else {
                // The re-key's `superseded` and `retired` records (D-3331;
                // WP-12c) are read, and change no rung: the census's epoch
                // installed with them no longer lists either key, so neither
                // is a stored strike here.
                // An `event` and a `stopped` record (D-3355) are read too:
                // a stopped key is retired with no successor, so it moves
                // no rung, and the event is its underlying's, not a key's.
                pull::expiry_alias::AliasRecord::parse(line).is_some()
                    || pull::gdfl_lineage::SupersededRecord::parse(line).is_some()
                    || pull::gdfl_lineage::RetiredRecord::parse(line).is_some()
                    || pull::gdfl_lineage::EventRecord::parse(line).is_some()
                    || pull::gdfl_lineage::StoppedRecord::parse(line).is_some()
            };
            if read {
                index.records = index.records.saturating_add(1);
            } else {
                index.unread = index.unread.saturating_add(1);
            }
        }
        index
    }

    /// Files one record half: the key in `map`, and its ex-date among the
    /// underlying's.
    fn put(
        &mut self,
        map: Map,
        underlying: Symbol,
        (expiry, strike, call): (Expiry, i64, bool),
        ex: pull::session::Day,
    ) {
        let ex = i64::from(ex.days_from_epoch());
        let marks = match map {
            Map::Renamed => &mut self.renamed,
            Map::Reused => &mut self.reused,
            Map::Listed => &mut self.listed,
        };
        marks
            .entry((underlying, expiry))
            .or_default()
            .insert((strike, call, ex));
        let days = self.adjusted.entry(underlying).or_default();
        if let Err(at) = days.binary_search(&ex) {
            days.insert(at, ex);
        }
    }

    /// Marks each strike side of `underlying` `expiry` with the adjustments
    /// that reached its key (the module header's rule): O(strikes ×
    /// ex-dates of the underlying), a few per share in its whole history,
    /// plus one `relisted` probe per newly listed key.
    ///
    /// `relisted(strike, call, from)` answers the first stored stamp of that
    /// key at or after `from` (microseconds), or `None`.
    fn mark(
        &self,
        underlying: Symbol,
        expiry: Expiry,
        strikes: &mut [Strike],
        relisted: &mut dyn FnMut(i64, bool, i64) -> Option<i64>,
    ) {
        let Some(days) = self.adjusted.get(&underlying) else {
            return;
        };
        let at = (underlying, expiry);
        let (renamed, reused, listed) = (
            self.renamed.get(&at),
            self.reused.get(&at),
            self.listed.get(&at),
        );
        let names = |map: Option<&HashSet<(i64, bool, i64)>>, key| {
            map.is_some_and(|set| set.contains(&key))
        };
        for strike in strikes.iter_mut() {
            for (side, call) in [(&mut strike.call, true), (&mut strike.put, false)] {
                for &ex in days {
                    let key = (strike.paisa, call, ex);
                    // The first instant of the ex-date's IST day.
                    let from = ex_start_micros(ex);
                    let before = side.first.is_some_and(|first| first < from);
                    if !before && !names(renamed, key) {
                        continue;
                    }
                    let held = if names(reused, key) {
                        Some(from)
                    } else if names(listed, key) {
                        relisted(strike.paisa, call, from)
                    } else {
                        None
                    };
                    side.adjusted.push((ex, held));
                }
            }
        }
    }
}

/// Which of a [`LineageIndex`]'s maps a record half goes to.
#[derive(Debug, Clone, Copy)]
enum Map {
    /// [`LineageIndex::renamed`].
    Renamed,
    /// [`LineageIndex::reused`].
    Reused,
    /// [`LineageIndex::listed`].
    Listed,
}

/// The first instant of the IST day numbered `day`, in microseconds.
const fn ex_start_micros(day: i64) -> i64 {
    (day * SECONDS_PER_DAY - IST_OFFSET_SECS) * MICROS
}

/// The month after `month`, or `None` past the calendar's end.
fn next_month(month: YearMonth) -> Option<YearMonth> {
    let (year, number) = if month.month() == 12 {
        (month.year().checked_add(1)?, 1)
    } else {
        (month.year(), month.month() + 1)
    };
    YearMonth::new(year, number).ok()
}

/// Where one option key's bars are filed: everything of its month files'
/// address but the month.
struct OptionKey<'a> {
    root: &'a std::path::Path,
    feed: Vendor,
    exchange: &'a str,
    underlying: &'a str,
    contract: Contract,
    timeframe: Timeframe,
}

/// The first stamp at or after `from` that `key` holds, read from its month
/// files from `from`'s month through `expiry`'s, the first month that holds
/// one answering: one `first_at_or_after` lookup and one record per month
/// opened (`/spot.json`'s bound). An absent month is passed over; a month
/// that would not read, or whose record there is not at or after `from` (the
/// zero-filled extent `store::file` documents), is a named fault and answers
/// `None`, which leaves the key out of the chain rather than guessed in.
fn first_stored_from(
    key: &OptionKey<'_>,
    from: i64,
    expiry: Expiry,
    faults: &mut Vec<String>,
) -> Option<i64> {
    let last = YearMonth::new(expiry.year(), expiry.month()).ok()?;
    let mut month = month_of(from)?;
    while month <= last {
        let opened = crate::bars::open_classified(
            key.root,
            PathParts {
                vendor: key.feed,
                exchange: key.exchange,
                segment: Segment::Fno.as_str(),
                symbol: key.underlying,
                contract: Some(key.contract),
                timeframe: key.timeframe,
                month,
                file: store::path::FileKind::Bars,
            },
        );
        match opened {
            Ok(file) => {
                let read = file.first_at_or_after(from).and_then(|at| {
                    if at < file.header().n_valid {
                        file.read_record(at).map(|bar| Some(bar.ts_micros))
                    } else {
                        Ok(None)
                    }
                });
                match read {
                    Ok(Some(ts)) if ts >= from => return Some(ts),
                    Ok(Some(ts)) => {
                        faults.push(format!(
                            "{}: the record found at or after t={} is stamped {ts} us, before it",
                            file.path().display(),
                            from.div_euclid(MICROS)
                        ));
                        return None;
                    }
                    Ok(None) => {}
                    Err(why) => {
                        faults.push(format!("{}: {why}", file.path().display()));
                        return None;
                    }
                }
            }
            Err(refusal) => match refusal.kind {
                OpenRefusalKind::Absent => {}
                OpenRefusalKind::Invalid | OpenRefusalKind::Unreadable => {
                    faults.push(refusal.message);
                    return None;
                }
            },
        }
        month = next_month(month)?;
    }
    None
}

/// An option contract's expiry, strike (paisa) and `true` for a call, or
/// `None` for anything else.
fn key_of(contract: Contract) -> Option<(Expiry, i64, bool)> {
    option_parts(contract.as_str())
        .map(|(expiry, strike, side)| (expiry, strike, side == OptionSide::Call))
}

/// The ledger the process last read: its path, stamp and index.
type LineageKept = (
    std::path::PathBuf,
    u64,
    std::time::SystemTime,
    Arc<LineageIndex>,
);

/// The process's one kept ledger read.
static LINEAGE_KEPT: Mutex<Option<LineageKept>> = Mutex::new(None);

/// The lineage ledger of `feed` under `root`, read when its stamp moved.
fn lineage_of(root: &std::path::Path, feed: Vendor) -> Lineage {
    if feed != Vendor::Gdfl {
        return Lineage::NotKept;
    }
    let path = pull::gdfl_lineage::lineage_path(root);
    let meta = match std::fs::metadata(&path) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Lineage::Absent,
        Err(why) => return Lineage::Unreadable(format!("{}: {why}", path.display())),
    };
    if meta.len() > MAX_LINEAGE_BYTES {
        return Lineage::Unreadable(format!(
            "{}: {} bytes, past the {MAX_LINEAGE_BYTES} this route reads",
            path.display(),
            meta.len()
        ));
    }
    let modified = meta.modified().ok();
    let mut kept = LINEAGE_KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    if let (Some(modified), Some((was_path, len, was_modified, index))) = (modified, kept.as_ref())
        && *was_path == path
        && *len == meta.len()
        && *was_modified == modified
    {
        return Lineage::Read(Arc::clone(index));
    }
    let text = match std::fs::read(&path).map(String::from_utf8) {
        Ok(Ok(text)) => text,
        Ok(Err(_)) => return Lineage::Unreadable(format!("{}: not UTF-8 text", path.display())),
        Err(why) => return Lineage::Unreadable(format!("{}: {why}", path.display())),
    };
    let index = Arc::new(LineageIndex::of(&text));
    if let Some(modified) = modified {
        *kept = Some((path, meta.len(), modified, Arc::clone(&index)));
    }
    Lineage::Read(index)
}

/// The underlying's month files this request opened, by month.
struct Months<'a> {
    root: &'a std::path::Path,
    feed: Vendor,
    exchange: &'a str,
    segment: &'a str,
    key: &'a str,
    timeframe: Timeframe,
    opened: Vec<(YearMonth, Result<BarFile, Spot>)>,
}

impl Months<'_> {
    /// The spot at `at` under `asof`, opening `at`'s month file the first
    /// time it is asked.
    fn spot(&mut self, at: i64, asof: AsOf, faults: &mut Vec<String>) -> Result<Spot, String> {
        let Some(month) = month_of(at) else {
            return Err(format!(
                "at={} is not an instant any calendar can place",
                at.div_euclid(MICROS)
            ));
        };
        if !self.opened.iter().any(|(held, _)| *held == month) {
            let opened = match crate::bars::open_classified(
                self.root,
                PathParts {
                    vendor: self.feed,
                    exchange: self.exchange,
                    segment: self.segment,
                    symbol: self.key,
                    contract: None,
                    timeframe: self.timeframe,
                    month,
                    file: store::path::FileKind::Bars,
                },
            ) {
                Ok(file) => Ok(file),
                Err(refusal) => match refusal.kind {
                    OpenRefusalKind::Invalid => return Err(refusal.message),
                    OpenRefusalKind::Absent => Err(Spot::NoFile),
                    OpenRefusalKind::Unreadable => {
                        faults.push(refusal.message);
                        Err(Spot::Unreadable)
                    }
                },
            };
            self.opened.push((month, opened));
        }
        let Some((_, opened)) = self.opened.iter().find(|(held, _)| *held == month) else {
            return Ok(Spot::Unreadable);
        };
        Ok(match opened {
            Ok(file) => spot_at(file, month, at, asof).unwrap_or_else(|why| {
                faults.push(format!(
                    "{} t={}: {why}",
                    file.path().display(),
                    at.div_euclid(MICROS)
                ));
                Spot::Unreadable
            }),
            Err(absent) => absent.clone(),
        })
    }
}

/// The census state as this request found it, beside the refusal it puts on
/// every instant it covers.
struct CensusView {
    state: &'static str,
    generation: Option<u64>,
    /// What loading stepped over, when the census is a recovered one.
    degraded: Option<String>,
    behind_days: u64,
    behind_from: Option<pull::session::Day>,
    journal_unread: Option<String>,
}

impl CensusView {
    /// The census refusal at `at` for `expiry`, if any.
    fn refusal(&self, at: i64, expiry: pull::session::Day) -> Option<(&'static str, String)> {
        if self.state == "absent" {
            return Some((
                "census_absent",
                "this feed has no census, so no strike can be counted".to_owned(),
            ));
        }
        if let Some(why) = &self.journal_unread {
            return Some((
                "census_state_unknown",
                format!(
                    "the import journal that says whether the census is behind would not read: {why}"
                ),
            ));
        }
        if let Some(why) = &self.degraded {
            return Some((
                "census_degraded",
                format!(
                    "the census was recovered from a damaged file, so it is an earlier generation and \
                     strikes a later one had counted may be missing; no ladder is counted over it: {why}"
                ),
            ));
        }
        let from = self.behind_from.filter(|_| self.behind_days > 0)?;
        let from_day = i64::from(from.days_from_epoch());
        let expiry_day = i64::from(expiry.days_from_epoch());
        (ist_day(at) >= from_day && expiry_day >= from_day).then(|| {
            (
                "census_behind",
                format!(
                    "the census is behind its import by {} day(s) from {from}: strikes first stored \
                     from then are not counted yet, so the ladder at this instant would be truncated \
                     (D-3327, D-3329)",
                    self.behind_days
                ),
            )
        })
    }
}

/// The whole answer, off the async workers.
#[expect(
    clippy::too_many_lines,
    reason = "one answer in the order the wire carries it: the census, the chain, the \
              spot's files, then each instant"
)]
fn answer(site: &Site, query: &str) -> Answered {
    let asked = match Asked::parse(query) {
        Ok(asked) => asked,
        Err(why) => return bad_request(&why),
    };
    let (_stamps, now) = vendor_census(site, asked.feed);
    let Some(census) = now.census() else {
        return bad_request("no census row for this feed");
    };
    if let census::Census::Unreadable { .. } = census.state {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            format!(
                r#"{{"error":"census_unreadable","why":{}}}"#,
                render::json_string(&census.note())
            ),
        );
    }
    let behind = super::census_behind(site, asked.feed);
    let state = census.state_word(behind.days);
    // THE CHAIN OF THIS ONE EXPIRY, BY PROBE (D-3329): the facet index holds
    // each contract's earliest stamp, moved forward by appended entries.
    let probed = super::facets::index(site, asked.feed, Segment::Fno, now.slot(), |index| {
        let exchange = if let Some(exchange) = asked.exchange {
            Some(exchange)
        } else {
            let held: Vec<Exchange> = [Exchange::Nse, Exchange::Bse]
                .into_iter()
                .filter(|exchange| index.holds(*exchange, asked.underlying))
                .collect();
            if held.len() > 1 {
                return Err(format!(
                    "{} is held on NSE and BSE; name one with exchange=",
                    asked.underlying.as_str()
                ));
            }
            held.first().copied()
        };
        let options = exchange.and_then(|exchange| {
            index.expiry_options(exchange, asked.underlying, asked.expiry_day)
        });
        Ok((index.generation(), exchange, options))
    });
    let (generation, exchange, options) = match probed {
        Ok(probed) => probed,
        Err(why) => return bad_request(&why),
    };
    let view = CensusView {
        state,
        generation: matches!(census.state, census::Census::Held { .. }).then_some(generation),
        degraded: census.degraded(),
        behind_days: behind.days,
        behind_from: behind.from,
        journal_unread: behind.unread.clone(),
    };
    let lineage = lineage_of(&site.store_root, asked.feed);
    let mut strikes = strikes_of(options.as_deref().unwrap_or(&[]));
    let mut faults = Vec::new();
    if let (Lineage::Read(index), Some(exchange)) = (&lineage, exchange) {
        let mut relisted = |paisa: i64, call: bool, from: i64| {
            let leg = if call { "CE" } else { "PE" };
            let key = OptionKey {
                root: &site.store_root,
                feed: asked.feed,
                exchange: exchange.as_str(),
                underlying: asked.underlying.as_str(),
                contract: Contract::parse(&format!("{}-{paisa}-{leg}", asked.expiry))?,
                timeframe: asked.timeframe,
            };
            first_stored_from(&key, from, asked.expiry, &mut faults)
        };
        index.mark(asked.underlying, asked.expiry, &mut strikes, &mut relisted);
    }
    let resolved = brutex_core::alias::option_underlying(asked.underlying.as_str());
    let mut months = exchange.map(|exchange| Months {
        root: &site.store_root,
        feed: asked.feed,
        exchange: exchange.as_str(),
        segment: resolved.segment.as_str(),
        key: resolved.key,
        timeframe: asked.timeframe,
        opened: Vec::new(),
    });
    let mut chains = String::with_capacity(asked.times.len() * 256 + 2);
    chains.push('[');
    for (n, &at) in asked.times.iter().enumerate() {
        if n > 0 {
            chains.push(',');
        }
        // NO EXCHANGE HOLDS THE UNDERLYING: no spot is read, rather than one
        // read on an exchange the census never named (the instant is refused
        // below for its census, whatever a spot would have said).
        let spot = match months.as_mut() {
            Some(months) => match months.spot(at, asked.asof, &mut faults) {
                Ok(spot) => Some(spot),
                Err(why) => return bad_request(&why),
            },
            None => None,
        };
        let held = if options.is_some() {
            view.refusal(at, asked.expiry_day)
        } else {
            view.refusal(at, asked.expiry_day).or_else(|| {
                Some((
                    "expiry_not_in_census",
                    format!(
                        "the census holds no option of {} expiring {}",
                        asked.underlying.as_str(),
                        asked.expiry
                    ),
                ))
            })
        };
        let money = match (&held, spot.as_ref().and_then(Spot::close)) {
            (Some((why, detail)), _) => Err((*why, detail.clone())),
            (None, Some(close)) => Ok(close),
            (None, None) => Err((
                spot.as_ref()
                    .and_then(Spot::why)
                    .unwrap_or("spot_unreadable"),
                "no spot at this instant, so there is no money to count rungs from".to_owned(),
            )),
        };
        let ladder = ladder(&strikes, at, money);
        render_instant(&mut chains, &asked, &strikes, &ladder, at, spot.as_ref());
    }
    chains.push(']');
    let (lineage_state, lineage_records, lineage_unread, lineage_error) = match &lineage {
        Lineage::NotKept => ("not_kept", 0, 0, None),
        Lineage::Absent => ("absent", 0, 0, None),
        Lineage::Read(index) => (
            if index.unread > 0 { "partial" } else { "read" },
            index.records,
            index.unread,
            None,
        ),
        Lineage::Unreadable(why) => ("unreadable", 0, 0, Some(why.as_str())),
    };
    let body = format!(
        r#"{{"feed":{},"exchange":{},"underlying":{},"expiry":{},"timeframe":{},"basis":"stored_by_then","basis_note":{},"generation":{},"census":{{"state":{},"degraded":{},"behind_days":{},"behind_from":{}}},"lineage":{{"state":{},"records":{lineage_records},"unread_lines":{lineage_unread},"error":{}}},"spot_series":{{"segment":{},"key":{},"asof":{}}},"faults":{},"chains":{chains}}}"#,
        render::json_string(asked.feed.as_str()),
        exchange.map_or_else(|| "null".to_owned(), |e| render::json_string(e.as_str())),
        render::json_string(asked.underlying.as_str()),
        render::json_string(&asked.expiry.to_string()),
        render::json_string(asked.timeframe.as_str()),
        render::json_string(BASIS_NOTE),
        view.generation
            .map_or_else(|| "null".to_owned(), |g| g.to_string()),
        render::json_string(view.state),
        view.degraded
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        view.behind_days,
        view.behind_from.map_or_else(
            || "null".to_owned(),
            |day| render::json_string(&day.to_string())
        ),
        render::json_string(lineage_state),
        lineage_error.map_or_else(|| "null".to_owned(), render::json_string),
        render::json_string(resolved.segment.as_str()),
        render::json_string(resolved.key),
        render::json_string(asked.asof.as_str()),
        if faults.is_empty() {
            "null".to_owned()
        } else {
            render::json_string(&faults.join("; "))
        },
    );
    let status = if faults.is_empty() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json_headers(), body)
}

/// One instant's object.
fn render_instant(
    out: &mut String,
    asked: &Asked,
    strikes: &[Strike],
    ladder: &Ladder,
    at: i64,
    spot: Option<&Spot>,
) {
    let strike_at = |position: usize| ladder.stored.get(position).and_then(|&n| strikes.get(n));
    let atm = ladder.atm.and_then(strike_at).map(|strike| strike.paisa);
    let (why, detail) = ladder
        .refusal
        .as_ref()
        .map_or(("null".to_owned(), "null".to_owned()), |(why, detail)| {
            (render::json_string(why), render::json_string(detail))
        });
    let _ = write!(
        out,
        r#"{{"at":{},{},"spot_why":{},"n_strikes":{},"step_local":{},"atm":{},"refusal":{why},"detail":{detail}"#,
        at.div_euclid(MICROS),
        spot_fields("spot", spot.and_then(Spot::read), at),
        spot.and_then(Spot::why)
            .map_or_else(|| "null".to_owned(), render::json_string),
        ladder.stored.len(),
        ladder
            .step_local
            .map_or_else(|| "null".to_owned(), |step| step.to_string()),
        atm.map_or_else(|| "null".to_owned(), |paisa| paisa.to_string()),
    );
    // `k` of the strike at `position` in the stored chain, when ATM is known.
    let k_of = |position: usize| {
        ladder.atm.and_then(|money| {
            let position = i64::try_from(position).ok()?;
            let money = i64::try_from(money).ok()?;
            Some(position - money)
        })
    };
    if let Some((paisa, side)) = asked.pick {
        let call = side == OptionSide::Call;
        let position = ladder
            .stored
            .iter()
            .position(|&n| strikes.get(n).is_some_and(|strike| strike.paisa == paisa));
        let (k, rung_word, why) = match position {
            None => (
                "null".to_owned(),
                "null".to_owned(),
                r#""strike_not_stored_by_then""#.to_owned(),
            ),
            Some(position) => match k_of(position) {
                Some(k) => (
                    k.to_string(),
                    render::json_string(&rung(k, call)),
                    "null".to_owned(),
                ),
                None => ("null".to_owned(), "null".to_owned(), "null".to_owned()),
            },
        };
        let _ = write!(
            out,
            r#","pick":{{"strike":{paisa},"side":{},"k":{k},"rung":{rung_word},"why":{why}}}}}"#,
            render::json_string(if call { "CE" } else { "PE" }),
        );
    } else {
        out.push_str(r#","strikes":["#);
        for (position, &n) in ladder.stored.iter().enumerate() {
            let Some(strike) = strikes.get(n) else {
                continue;
            };
            if position > 0 {
                out.push(',');
            }
            let k = k_of(position);
            let first = |side: &Side| {
                side.first_by(at)
                    .map_or_else(|| "null".to_owned(), |secs| secs.to_string())
            };
            let named = |call: bool| {
                k.map_or_else(
                    || "null".to_owned(),
                    |k| render::json_string(&rung(k, call)),
                )
            };
            let _ = write!(
                out,
                r#"{{"strike":{},"k":{},"ce":{},"pe":{},"ce_first":{},"pe_first":{}}}"#,
                strike.paisa,
                k.map_or_else(|| "null".to_owned(), |k| k.to_string()),
                named(true),
                named(false),
                first(&strike.call),
                first(&strike.put),
            );
        }
        out.push_str("]}");
    }
}

/// `GET /chain.json`. See the module header.
pub(super) async fn chain_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    // OFF THE ASYNC WORKERS (DB-066, DB-G80): a census miss reads a
    // manifest, the ledger and the spot's month files are blocking reads.
    match crate::detail::run_store_read(move || answer(&site, &query)).await {
        Ok(answered) => answered,
        Err(why) => not_admitted("a chain read", &why),
    }
}

#[cfg(test)]
#[path = "chain_route_tests.rs"]
mod tests;
