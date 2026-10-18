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
//! their current totals, so a client merges them by key. A generation older
//! than the census's last whole read -- a restart, a tail read whole again
//! past its floor -- is answered from the FILE, read back by position
//! (`census::Part::changed_since`; R3-09, RA-008), so a tab kept open across
//! them refreshes by a delta. One the file cannot answer (another file now
//! stands at the path: a new epoch, a compaction, a rename; or more than
//! `census::SINCE_READ_BACK_MOST` entries since) is refused `410
//! since_too_old`, and the client loads the selection afresh; never a
//! silent partial delta.
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
use std::sync::{Arc, Mutex, PoisonError, RwLock};

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

/// One (month, rung) of the coverage roll-up (`census_rollup.rs`; R3-09):
/// how many census keys hold that month at that rung, their rows summed,
/// and the earliest first and the latest last stamp among them. Kept per
/// underlying and per segment by the same `apply` the nodes are, so a
/// roll-up is read off the index and never walks the census.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MonthCell {
    /// Census keys (instrument-months) at this month and rung.
    pub(crate) keys: u64,
    /// Their rows summed.
    pub(crate) rows: u64,
    /// The earliest first stamp, in microseconds.
    pub(crate) first: Option<i64>,
    /// The latest last stamp, in microseconds.
    pub(crate) last: Option<i64>,
}

impl MonthCell {
    /// One entry moved from `old` (or new) to `new`, exactly as
    /// [`Agg::apply`] moves a node: a key is counted once, when first seen;
    /// rows move by the difference; the first stamp is a minimum and the
    /// last a maximum, both exact because a key's rows and last stamp only
    /// grow and its first stamp is fixed once written.
    fn apply(&mut self, old: Option<&Held>, new: &Held) {
        let entry = &new.entry;
        if old.is_none() {
            self.keys = self.keys.saturating_add(1);
        }
        self.rows = self
            .rows
            .saturating_sub(old.map_or(0, |was| was.entry.rows))
            .saturating_add(entry.rows);
        self.first = Some(self.first.map_or(entry.first_ts_micros, |first| {
            first.min(entry.first_ts_micros)
        }));
        self.last = Some(
            self.last
                .map_or(entry.last_ts_micros, |last| last.max(entry.last_ts_micros)),
        );
    }
}

/// Every (month, rung) cell of one roll-up, oldest month first, finest rung
/// first within a month.
pub(crate) type Cells = BTreeMap<(YearMonth, Timeframe), MonthCell>;

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
    /// Distinct option strikes among `contracts`, counted by `apply` as each
    /// is first seen, so the underlying's expiries answer reads a counter
    /// rather than collecting every contract of every expiry into a set per
    /// request (`cloud/attack-o1-20261016`).
    strikes: u64,
}

/// One underlying: its months and, in FNO, its expiries.
#[derive(Debug, Clone, Default)]
pub(crate) struct Underlying {
    pub(crate) agg: Agg,
    months: BTreeMap<YearMonth, Agg>,
    expiries: BTreeMap<Day, Expiry>,
    /// Keys whose contract text is not one this index can read.
    unread_contracts: u64,
    /// The coverage roll-up of this underlying, per (month, rung): what
    /// `/census/rollup.json` reads (R3-09).
    cells: Cells,
    /// Distinct contracts across every expiry, counted by `apply` as each is
    /// first seen: the underlyings answer read it by walking every expiry of
    /// every underlying on every request (`cloud/attack-o1-20261016`). A key
    /// only ever grows under `apply`, so a counter never has to come down.
    contracts: u64,
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
    /// The coverage roll-up of the whole segment, per (month, rung): what
    /// `/census/rollup.json` reads without an underlying (R3-09).
    cells: Cells,
    /// How long a test says this index takes to drop ([`SLOW_DROP_KEY`]).
    #[cfg(test)]
    drop_pause: DropPause,
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

    /// The whole segment's coverage roll-up per (month, rung): one field
    /// read, whatever the census holds (`census_rollup.rs`; R3-09).
    pub(crate) const fn segment_cells(&self) -> &Cells {
        &self.cells
    }

    /// One underlying's coverage roll-up per (month, rung), or `None` when
    /// the segment holds no such underlying on `exchange`: one map probe
    /// (`census_rollup.rs`; R3-09).
    pub(crate) fn underlying_cells(&self, exchange: Exchange, symbol: Symbol) -> Option<&Cells> {
        self.underlyings
            .get(&(exchange, symbol))
            .map(|under| &under.cells)
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
        self.cells
            .entry((key.month, key.timeframe))
            .or_default()
            .apply(old, new);
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
        under
            .cells
            .entry((key.month, key.timeframe))
            .or_default()
            .apply(old, new);
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
        // THE TWO COUNTS THE ANSWERS READ, KEPT WHERE A CONTRACT IS FIRST
        // SEEN: a strike is new when neither side of it was held before.
        let fresh = !chain.contracts.contains_key(&id);
        if fresh
            && let ContractId::Option { strike, call } = id
            && !chain.contracts.contains_key(&ContractId::Option {
                strike,
                call: !call,
            })
        {
            chain.strikes = chain.strikes.saturating_add(1);
        }
        chain.contracts.entry(id).or_default().apply(old, new);
        if fresh {
            under.contracts = under.contracts.saturating_add(1);
        }
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

/// One (feed, segment)'s index and the locks its moves and builds are made
/// under. See [`IndexSlot`].
type IndexCell = Arc<IndexSlot>;

/// One (feed, segment)'s index, behind the lock its moves are made under:
/// a READ lock for every answer that finds it current, so concurrent answers
/// render side by side instead of queueing behind one another (R3-09: sixty
/// clients on NIFTY's FNO underlying queued to 141 ms behind one `Mutex`);
/// the write lock only to move it forward by appended entries, and to swap
/// in a whole build.
///
/// # A whole build is made OFF the index's lock (R3-09)
///
/// A build over every key of the segment (the first after a start, or after
/// a new epoch, a compaction or a file replaced by rename) took the write
/// lock for its whole length: 2.6 s for GDFL's 2,068,946 FNO entries, during
/// which every answer of the segment queued behind it -- the ones the
/// previous read could answer included. It is now made under `building`
/// alone (one build at a time, so a burst after a restart builds once) with
/// the index free, swapped in under a moment's write lock, and `pending`
/// says one is on its way: what [`building`] and [`index_or_previous`] read
/// to answer "warming" or from the previous index at once instead of
/// waiting.
///
/// # Pending from BEFORE the read is published (R3-09 repair 1)
///
/// `pending` was set by the build itself, after it took `building`. The
/// background re-read publishes the new census read first and builds the
/// index after the import journal's count and the index map (283 to
/// 1,657 ms on the shadow of the real store): inside that window a request
/// handed the new read found nothing pending, built the whole index itself,
/// or waited on `building` for the re-read's build, 7.55 s and 12.21 s
/// measured. Now the re-read holds `pending` before it publishes
/// ([`hold_pending`]), a whole build holds it while it runs, and a request
/// that finds the index behind its read with nothing pending starts a build
/// on a thread of its own ([`kick`]). No request thread takes `building`
/// once the census runs in the background.
///
/// # The old index is dropped OFF the lock (R3-09 repair 1)
///
/// The swap moved the new index in under the write lock and dropped the old
/// one there: 0.8 to 1.9 s for GDFL's 2,068,946-entry FNO index, every
/// reader of the segment queued on the read lock meanwhile. Only the move is
/// made under the lock now; the old index is retired to a thread of its own
/// ([`super::census_cache::retire`]).
#[derive(Debug, Default)]
pub(super) struct IndexSlot {
    index: RwLock<Index>,
    building: Mutex<()>,
    pending: Mutex<Pending>,
}

/// Who is bringing one index forward, and since when. See [`IndexSlot`].
#[derive(Debug, Default)]
struct Pending {
    /// How many hold it: a census re-read about to publish a read the index
    /// does not reflect yet ([`hold_pending`]), a whole build in [`index`],
    /// a build a request started ([`kick`]).
    holders: u32,
    /// When the first of them took it; `None` while nobody holds it.
    since: Option<std::time::SystemTime>,
}

/// One holder of an index's [`Pending`], released when dropped -- by return
/// or by panic, so a build that died never leaves the routes answering
/// "warming" for the life of the process.
#[derive(Debug)]
pub(crate) struct PendingHold(IndexCell);

impl PendingHold {
    /// Holds `cell`'s pending, beside any other holder.
    fn of(cell: &IndexCell) -> Self {
        let mut pending = cell.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.holders = pending.holders.saturating_add(1);
        let _first = pending.since.get_or_insert_with(std::time::SystemTime::now);
        Self(Arc::clone(cell))
    }

    /// Holds `cell`'s pending if nobody does; else when the holder took it.
    fn if_idle(cell: &IndexCell) -> Result<Self, std::time::SystemTime> {
        let mut pending = cell.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(since) = pending.since.filter(|_| pending.holders > 0) {
            return Err(since);
        }
        let since = std::time::SystemTime::now();
        *pending = Pending {
            holders: 1,
            since: Some(since),
        };
        Ok(Self(Arc::clone(cell)))
    }

    /// When the pending this holds was first taken.
    fn since(&self) -> std::time::SystemTime {
        self.0
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .since
            .unwrap_or_else(std::time::SystemTime::now)
    }
}

impl Drop for PendingHold {
    fn drop(&mut self) {
        let mut pending = self
            .0
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        pending.holders = pending.holders.saturating_sub(1);
        if pending.holders == 0 {
            pending.since = None;
        }
    }
}

/// Holds every segment's index of `vendor` pending: what a census re-read
/// takes BEFORE it publishes a read, and drops once the indexes are brought
/// to it, so a request handed the new read meanwhile answers from the
/// previous index or `503 index_warming` and never builds or waits
/// ([`IndexSlot`]).
pub(crate) fn hold_pending(site: &Site, vendor: Vendor) -> Vec<PendingHold> {
    SEGMENTS
        .into_iter()
        .map(|segment| PendingHold::of(&cell_of(site, vendor, segment)))
        .collect()
}

/// The cell of (`vendor`, `segment`), made on first use.
fn cell_of(site: &Site, vendor: Vendor, segment: Segment) -> IndexCell {
    let mut cells = site
        .census_wire
        .facets
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Arc::clone(cells.entry((vendor, segment)).or_default())
}

/// Whether `index` answers a request handed `slot`: it reflects `slot`, or a
/// LATER read than it ([`later`]).
fn answers(site: &Site, vendor: Vendor, index: &Index, slot: &Arc<Slot>) -> bool {
    index
        .reflects
        .as_ref()
        .is_some_and(|reflects| Arc::ptr_eq(reflects, slot) || later(site, vendor, reflects, slot))
}

/// `segment`'s index as of `slot`, built or moved forward, under its lock.
///
/// Moved forward when the index reflects an earlier read of the same census
/// and that census still holds the appends since (`changed_since`); built
/// from the census otherwise, off the index's lock ([`IndexSlot`]). Either
/// way the sums equal a fresh build over `slot`'s census:
/// `facets::a_moved_index_equals_a_fresh_build` holds it.
///
/// WAITS for a whole build: what the census re-read and a request's
/// [`kick`] run on threads of their own, and what a one-shot caller (no
/// background census, `census_cache::start` never ran) runs on its own. A
/// route never calls it while the census runs in the background
/// ([`index_or_previous`], [`building`]); `/chain.json` still does (its
/// owner's follow-up, named in `docs/06-limits.md`).
pub(crate) fn index<T>(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    slot: &Arc<Slot>,
    read: impl FnOnce(&Index) -> T,
) -> T {
    let cell = cell_of(site, vendor, segment);
    {
        let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
        if answers(site, vendor, &held, slot) {
            return read(&held);
        }
    }
    // MOVED BY ITS TAIL, the entries found off the lock and applied under
    // the write lock: a mover that took it after another moved the index
    // finds it current, so a burst after a change moves it once; and never
    // moved BACK over a read older than the one another request moved it to
    // ([`later`]). Whatever the move displaced is dropped after the lock is
    // let go, never under it.
    if !moved_off_lock(site, vendor, &cell, segment, slot) {
        // BUILT WHOLE, OFF THE INDEX'S LOCK, ONE BUILD AT A TIME: a builder
        // that waited for another finds the index current and builds nothing.
        let _one = cell.building.lock().unwrap_or_else(PoisonError::into_inner);
        let current = {
            let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
            answers(site, vendor, &held, slot)
        };
        if !current {
            // PENDING WHILE IT RUNS, released by return or by panic.
            let _pending = PendingHold::of(&cell);
            #[cfg(test)]
            super::census_cache::slow_read::pause(&site.store_root.join(SLOW_BUILD_KEY));
            #[cfg(test)]
            let fresh = {
                let mut fresh = whole(segment, slot);
                fresh.drop_pause = DropPause(super::census_cache::slow_read::ms(
                    &site.store_root.join(SLOW_DROP_KEY),
                ));
                fresh
            };
            #[cfg(not(test))]
            let fresh = whole(segment, slot);
            // A MOMENT'S WRITE LOCK, TO MOVE A POINTER: the old index (two
            // million entries at GDFL's size) is dropped after the lock is
            // let go, on a thread of its own. A LATER READ MOVED IT MEANWHILE
            // (by its tail, while this build ran): kept, and the build
            // retired instead.
            let retired = {
                let mut held = cell.index.write().unwrap_or_else(PoisonError::into_inner);
                if answers(site, vendor, &held, slot) {
                    fresh
                } else {
                    std::mem::replace(&mut *held, fresh)
                }
            };
            super::census_cache::retire(Dismantled(retired));
        }
    }
    let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
    read(&held)
}

/// Brings `segment`'s index to `slot` ([`index`]), and says whether it
/// answers `slot` once that returns: what the census re-read runs on its own
/// thread for every segment before it lists the facet index `ready`.
pub(crate) fn bring(site: &Site, vendor: Vendor, segment: Segment, slot: &Arc<Slot>) -> bool {
    index(site, vendor, segment, slot, |index| {
        answers(site, vendor, index, slot)
    })
}

/// The key under a store root whose `census_cache::slow_read` pause slows
/// every whole facet-index build there: what a test stands a long build up
/// with. Not a path anything reads or writes.
#[cfg(test)]
pub(crate) const SLOW_BUILD_KEY: &str = "facets-whole-build-pause";

/// The key under a store root whose `census_cache::slow_read` milliseconds
/// every index built whole there takes to DROP: what a test stands a
/// two-million-entry index's drop up with (R3-09 repair 1). Not a path
/// anything reads or writes.
#[cfg(test)]
pub(crate) const SLOW_DROP_KEY: &str = "facets-index-drop-pause";

/// A test's stand-in for the time a large index takes to drop: sleeps its
/// milliseconds when the index that carries it is dropped ([`SLOW_DROP_KEY`]).
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub(crate) struct DropPause(u64);

#[cfg(test)]
impl Drop for DropPause {
    fn drop(&mut self) {
        if self.0 > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.0));
        }
    }
}

/// When a build stands between a request handed `slot` and its answer: the
/// census re-read holds it pending from before it published the read, a
/// whole build of this cell is running, a request started one, or the warm
/// at start has not built this feed's indexes yet (`census_cache::start`).
/// `None` when nothing is bringing it forward.
fn pending(site: &Site, vendor: Vendor, cell: &IndexSlot) -> Option<std::time::SystemTime> {
    let held = {
        let pending = cell.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.since.filter(|_| pending.holders > 0)
    };
    held.or_else(|| super::census_cache::warming_since(site, super::census_cache::FACETS, vendor))
}

/// Milliseconds since `at`, saturated.
fn ms_since(at: std::time::SystemTime) -> u64 {
    u64::try_from(at.elapsed().unwrap_or_default().as_millis()).unwrap_or(u64::MAX)
}

/// Where a request handed `slot` stands with one index, found without
/// waiting for any build.
enum Stand {
    /// The index answers `slot` ([`answers`]).
    Current,
    /// A build is on its way, held since this instant: the request answers
    /// from the previous index, or `503 index_warming`.
    Pending(std::time::SystemTime),
    /// Nothing builds it and no thread could be started for it (the census
    /// is not in the background: a one-shot caller, a test without
    /// `census_cache::start`): the request builds it itself, which is slow
    /// and never wrong.
    Unattended,
}

/// Where a request handed `slot` stands: current; pending; current after a
/// move that needs no whole build (the appended entries since the index's
/// generation, or the empty index of a segment `slot` holds no part for),
/// made by the request when nothing is bringing the index forward; else
/// pending on a build it starts on a thread of its own ([`kick`]); else
/// unattended.
fn stand(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    cell: &IndexCell,
    slot: &Arc<Slot>,
) -> Stand {
    {
        let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
        if answers(site, vendor, &held, slot) {
            return Stand::Current;
        }
    }
    if let Some(since) = pending(site, vendor, cell) {
        return Stand::Pending(since);
    }
    // THE ONE MOVE A REQUEST MAKES: to the empty index of a segment its read
    // holds no part for, O(1). A move by appended entries (up to a quarter
    // of a million, read back from the file below the base) is made off the
    // request, by the build it starts.
    if part_of(slot, segment).is_none() && moved_off_lock(site, vendor, cell, segment, slot) {
        return Stand::Current;
    }
    kick(site, vendor, segment, cell, slot).map_or(Stand::Unattended, Stand::Pending)
}

/// Starts a build of this index on a thread of its own, for a request that
/// found it behind its read with nothing bringing it forward: a re-read
/// that ended without building it (it panicked, or the read it published
/// was not kept), or a census read on a request thread. Built over the read
/// the vendor's cell KEEPS where it keeps one (what every later request is
/// handed), else over the request's own. One at a time: a request that
/// finds one started is told when.
///
/// `None` when the census is not in the background (no thread may build
/// for the site), the read to build is a refused one (every route refuses
/// it before it reaches an index), or the thread could not be started; the
/// request then builds the index itself.
fn kick(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    cell: &IndexCell,
    slot: &Arc<Slot>,
) -> Option<std::time::SystemTime> {
    let loaded = super::census_cache::background_site(site)?;
    // NEVER A REFUSED READ: the kept one if it loaded, else the request's own
    // (a kept read refused after the request was handed a held one).
    let unreadable = |read: &Slot| matches!(read.census().state, census::Census::Unreadable { .. });
    let target = super::census_cache::kept_slot(site, vendor)
        .filter(|kept| !unreadable(kept))
        .unwrap_or_else(|| Arc::clone(slot));
    if unreadable(&target) {
        return None;
    }
    let hold = match PendingHold::if_idle(cell) {
        Ok(hold) => hold,
        Err(since) => return Some(since),
    };
    let since = hold.since();
    let spawned = std::thread::Builder::new()
        .name(format!(
            "brutex-facets-{}-{}",
            vendor.as_str(),
            segment.as_str()
        ))
        .spawn(move || {
            let _hold = hold;
            index(&loaded, vendor, segment, &target, |_| ());
        });
    match spawned {
        Ok(_detached) => {
            let _dropped_when_filtered = telemetry::emit_if!(
                telemetry::Level::Info,
                "api.census",
                "facet index rebuild started by a request",
                "feed" => telemetry::Value::Str(vendor.as_str()),
                "segment" => telemetry::Value::Str(segment.as_str()),
            );
            Some(since)
        }
        Err(why) => {
            warn_line!(
                "the {} {} facet index could not be rebuilt on a thread of its own ({why}); \
                 the request builds it itself",
                vendor.as_str(),
                segment.as_str()
            );
            None
        }
    }
}

/// Whether `index`, which does not answer `slot`, may answer for it as the
/// PREVIOUS index: it was built over a read, and that read holds a part for
/// `segment` whenever `slot`'s does. An index over a census with no part for
/// the segment -- unreadable, refused or absent -- is empty, and served for
/// a census that holds the segment it said "not held" and "0 underlyings"
/// for 3 to 5 s after a census recovered (R3-09 repair 1): that answer is
/// `503 index_warming` instead.
fn serves_as_previous(index: &Index, segment: Segment, slot: &Slot) -> bool {
    index.reflects.as_ref().is_some_and(|reflects| {
        part_of(reflects, segment).is_some() || part_of(slot, segment).is_none()
    })
}

/// What [`index_or_previous`] answered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answered {
    /// The index as of the request's read (or a later one, [`later`]).
    Current,
    /// The PREVIOUS index, while a whole build over the request's read runs:
    /// built from the read that began at `read_at`, and standing in since
    /// `stale_since` (when the build it waits for was first pending). The
    /// answer says so, as an answer from the previous census read does
    /// ([`Freshness::Stale`]).
    ///
    /// [`Freshness::Stale`]: super::census_cache::Freshness::Stale
    Previous {
        read_at: std::time::SystemTime,
        stale_since: std::time::SystemTime,
    },
}

impl Answered {
    /// The census freshness an answer from this index carries: the request's
    /// own, or, from the previous index, stale since the read it stands on.
    pub(crate) fn freshness(
        self,
        census: super::census_cache::Freshness,
    ) -> super::census_cache::Freshness {
        match self {
            Self::Current => census,
            Self::Previous {
                read_at,
                stale_since,
            } => super::census_cache::Freshness::Stale {
                read_at,
                stale_since: census
                    .stale_since()
                    .map_or(stale_since, |at| at.min(stale_since)),
            },
        }
    }
}

/// [`index`], unless a build stands between the request and its answer
/// (R3-09, RA-046): then the PREVIOUS index answers, said to be previous,
/// while the build runs; and with no previous index that can stand for the
/// request's read (the first build after a start, or an index over a census
/// that held nothing of the segment, [`serves_as_previous`]) the request is
/// told how long the build has been pending (`Err`, milliseconds), at once,
/// instead of waiting it out. Over 20 restarts of the shadow of the real
/// store the first roll-up of NIFTY's options waited 5.6 s on average for
/// the warm's FNO build; a compaction or a new epoch had every answer of the
/// segment wait for its rebuild.
///
/// What `/census/facets.json` and `/census/rollup.json` call. Never waits
/// for a build while the census runs in the background: one short lock, then
/// the index's read lock, which a build holds only to move a pointer
/// ([`IndexSlot`]); a request that finds the index behind with nothing
/// bringing it forward starts the build on a thread of its own ([`kick`]).
///
/// # Errors
///
/// The milliseconds the build this request would wait for has been pending.
pub(crate) fn index_or_previous<T>(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    slot: &Arc<Slot>,
    read: impl FnOnce(&Index) -> T,
) -> Result<(T, Answered), u64> {
    let cell = cell_of(site, vendor, segment);
    let since = match stand(site, vendor, segment, &cell, slot) {
        Stand::Current | Stand::Unattended => {
            return Ok((index(site, vendor, segment, slot, read), Answered::Current));
        }
        Stand::Pending(since) => since,
    };
    let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
    if answers(site, vendor, &held, slot) {
        return Ok((read(&held), Answered::Current));
    }
    if !serves_as_previous(&held, segment, slot) {
        return Err(ms_since(since));
    }
    match held.reflects.as_ref().map(|reflects| reflects.read_at()) {
        Some(read_at) => Ok((
            read(&held),
            Answered::Previous {
                read_at,
                stale_since: since,
            },
        )),
        None => Err(ms_since(since)),
    }
}

/// Whether a request handed `slot` would wait for a build of this index:
/// how long it has been pending, in milliseconds, else `None`. What an
/// answer that reaches the index by another path (`rows=census-tuples-v1`)
/// asks first. A request that finds the index behind with nothing bringing
/// it forward starts the build ([`kick`]) and is told so.
pub(crate) fn building(
    site: &Site,
    vendor: Vendor,
    segment: Segment,
    slot: &Arc<Slot>,
) -> Option<u64> {
    let cell = cell_of(site, vendor, segment);
    match stand(site, vendor, segment, &cell, slot) {
        Stand::Current | Stand::Unattended => None,
        Stand::Pending(since) => {
            let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
            (!answers(site, vendor, &held, slot)).then(|| ms_since(since))
        }
    }
}

/// `503 index_warming`, naming the index, how long its build has run, and
/// that this answer did not wait for it; `Retry-After: 1`.
pub(crate) fn warming_answer(
    headers: &mut axum::http::HeaderMap,
    vendor: Vendor,
    segment: Segment,
    since_ms: u64,
) -> (axum::http::StatusCode, String) {
    let _retry = headers.insert(
        axum::http::header::RETRY_AFTER,
        axum::http::HeaderValue::from_static("1"),
    );
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        format!(
            r#"{{"error":"index_warming","why":{},"cache":{},"feed":{},"segment":{},"since_ms":{since_ms}}}"#,
            render::json_string(&format!(
                "the {} {} facet index is being built over the census (begun {since_ms} ms \
                 ago: the warm at start, or a rebuild after a new census file), and this \
                 answer did not wait for it; ask again in a moment",
                vendor.as_str(),
                segment.as_str()
            )),
            render::json_string(super::census_cache::FACETS),
            render::json_string(vendor.as_str()),
            render::json_string(segment.as_str()),
        ),
    )
}

/// Whether the index reflects a LATER read than `slot`: the read
/// `vendor`'s cell keeps now, of a higher generation than `slot`'s, which the
/// cell no longer keeps. A request handed the previous read while a
/// background re-read ran (R3-09) and reaching the index just after that
/// re-read moved it on is answered from the moved index -- never older than
/// it asked, and self-consistent, since every answer carries the index's own
/// generation -- instead of building the index back over the older census:
/// a whole build on a request thread that the next current request would
/// undo. A slot of the same or a higher generation, or an index that
/// reflects a read the cell does not keep (a test's detached slot), is moved
/// or built as before.
fn later(site: &Site, vendor: Vendor, reflects: &Arc<Slot>, slot: &Arc<Slot>) -> bool {
    let generation = |slot: &Slot| slot.census().generation();
    generation(reflects) > generation(slot)
        && super::census_cache::keeps(site, vendor, reflects)
        && !super::census_cache::keeps(site, vendor, slot)
}

/// [`index`]'s update, on its own so a test drives it without a site: moved
/// by the appended entries where it can be, else built whole. [`index`]
/// itself builds whole off the index's lock instead ([`IndexSlot`]).
#[cfg(test)]
fn advance(index: &mut Index, segment: Segment, slot: &Arc<Slot>) {
    let mut displaced = None;
    if !moved_by_tail(index, segment, slot, &mut displaced) {
        *index = whole(segment, slot);
    }
}

/// Moves `index` to `slot` where that needs no whole build: it reflects
/// `slot` already, `slot` holds no part for `segment` (an empty index), or
/// the part continues the file the index was built from and still holds the
/// appends since its generation. `false` when only a whole build reaches it.
///
/// An index REPLACED by the empty one is handed back in `displaced`, for
/// the caller to drop after it lets the index's lock go: dropped under the
/// write lock, at GDFL's size it stalls every reader of the segment for a
/// second or more (R3-09 repair 1). The move a test drives without a site:
/// [`index`] makes the same one with the entries found off the lock
/// ([`moved_off_lock`]).
#[cfg(test)]
fn moved_by_tail(
    index: &mut Index,
    segment: Segment,
    slot: &Arc<Slot>,
    displaced: &mut Option<Index>,
) -> bool {
    if index
        .reflects
        .as_ref()
        .is_some_and(|reflects| Arc::ptr_eq(reflects, slot))
    {
        return true;
    }
    if part_of(slot, segment).is_none() {
        *displaced = Some(emptied(index, slot));
        return true;
    }
    let Some(before) = index.reflects.clone() else {
        return false;
    };
    let Some((part, was, changes)) = tail_of(&before, index.generation, segment, slot) else {
        return false;
    };
    apply_tail(index, segment, slot, part, was, &changes);
    true
}

/// Replaces `index` by the empty index over `slot` (a segment `slot` holds
/// no part for), handing back the index it replaced for the caller to drop
/// after it lets the lock go. O(1).
fn emptied(index: &mut Index, slot: &Arc<Slot>) -> Index {
    std::mem::replace(
        index,
        Index {
            reflects: Some(Arc::clone(slot)),
            ..Index::default()
        },
    )
}

/// What moves an index built over `before` at `generation` forward to
/// `slot` by its appended entries: `slot`'s part, `before`'s, and the
/// entries appended since. Found WITHOUT the index's lock (R3-09 repair 1):
/// below the part's base they are read back from the file by position, up
/// to `census::SINCE_READ_BACK_MOST` entries, I/O that was made under the
/// write lock with every reader of the segment queued behind it. `None`
/// when only a whole build reaches `slot`.
///
/// MOVED FORWARD ONLY ALONG ONE FILE: the parts must share their whole read,
/// or continue the same file (R3-09: a census read whole again past its tail
/// floor continues it, and the appends since the index's generation are
/// read back by position), or the generation the index stands at may name a
/// census of another file (a new epoch, a migration, a file replaced by
/// rename): built again then.
fn tail_of<'a>(
    before: &'a Slot,
    generation: u64,
    segment: Segment,
    slot: &'a Slot,
) -> Option<(&'a census::Part, &'a census::Part, census::Since<'a>)> {
    let part = part_of(slot, segment)?;
    let was = part_of(before, segment)?;
    if !part.continues_file(was) {
        return None;
    }
    let changes = part.changed_since(generation)?;
    Some((part, was, changes))
}

/// Applies the entries [`tail_of`] found to `index`, which then reflects
/// `slot`: O(entries appended), all in memory, under the write lock.
fn apply_tail(
    index: &mut Index,
    segment: Segment,
    slot: &Arc<Slot>,
    part: &census::Part,
    was: &census::Part,
    changes: &census::Since<'_>,
) {
    let mut seen: HashSet<EntryKey> = HashSet::with_capacity(changes.len());
    for (_, key) in changes.iter() {
        if key.segment != segment || !seen.insert(*key) {
            continue;
        }
        if let Some(now) = part.held(key) {
            index.apply(was.held(key).as_ref(), &now);
        }
    }
    index.reflects = Some(Arc::clone(slot));
    index.generation = part.header().generation;
}

/// Moves `cell`'s index to `slot` by its appended entries, found off the
/// lock ([`tail_of`]) and applied under it, or to the empty index of a
/// segment `slot` holds no part for: `true` when the index answers `slot`
/// after it. `false` when only a whole build reaches `slot`, or another
/// thread moved the index between the two locks (the caller builds, and
/// finds it current under `building` if that move reached `slot`).
fn moved_off_lock(
    site: &Site,
    vendor: Vendor,
    cell: &IndexSlot,
    segment: Segment,
    slot: &Arc<Slot>,
) -> bool {
    let (before, generation) = {
        let held = cell.index.read().unwrap_or_else(PoisonError::into_inner);
        if answers(site, vendor, &held, slot) {
            return true;
        }
        (held.reflects.clone(), held.generation)
    };
    if part_of(slot, segment).is_none() {
        let replaced = {
            let mut held = cell.index.write().unwrap_or_else(PoisonError::into_inner);
            (!answers(site, vendor, &held, slot)).then(|| emptied(&mut held, slot))
        };
        super::census_cache::retire_any(replaced.map(Dismantled));
        return true;
    }
    let Some(before) = before else {
        return false;
    };
    let Some((part, was, changes)) = tail_of(&before, generation, segment, slot) else {
        return false;
    };
    let mut held = cell.index.write().unwrap_or_else(PoisonError::into_inner);
    if answers(site, vendor, &held, slot) {
        return true;
    }
    // STILL THE INDEX THE TAIL WAS FOUND FOR, or another thread moved it
    // between the locks and these entries do not continue it.
    let same = held
        .reflects
        .as_ref()
        .is_some_and(|reflects| Arc::ptr_eq(reflects, &before))
        && held.generation == generation;
    if same {
        apply_tail(&mut held, segment, slot, part, was, &changes);
    }
    same
}

/// An index a swap replaced, on its way to the retire thread: dropped there
/// a little at a time ([`dismantle`]), never in one burst.
///
/// # Why not a plain drop on the retire thread (R3-09 repair 1, item 4)
///
/// Freeing GDFL's FNO index is about a million small frees (977 ms of the
/// retire thread in a `sample` of the shadow of the real store). Made in one
/// burst, every request of the process -- `/feeds.json`, which reads no
/// index, included -- stalled 67 to 138 ms at once, 10 to 20 ms after the
/// swap, once per census change (three changes, five client loops). Paced,
/// the allocator gets the time between steps back; and the census read the
/// index stood on is let go LAST, after the nodes, so a stall that is the
/// census's own free is seen apart from the index's.
pub(crate) struct Dismantled(Index);

impl Drop for Dismantled {
    fn drop(&mut self) {
        let _freed = dismantle(std::mem::take(&mut self.0), DISMANTLE_STEP, &mut || {
            std::thread::sleep(DISMANTLE_PAUSE);
        });
    }
}

/// Nodes the retire thread frees between two pauses ([`dismantle`]).
const DISMANTLE_STEP: usize = 8_192;

/// The retire thread's pause between two steps of [`dismantle`].
const DISMANTLE_PAUSE: std::time::Duration = std::time::Duration::from_millis(1);

/// Takes `index` apart: every expiry, every underlying's months and cells,
/// the segment's own sums, and LAST the census read it reflects, calling
/// `pause` after every `step` nodes freed. The nodes it freed, counted as
/// one per contract, month, expiry and underlying.
fn dismantle(index: Index, step: usize, pause: &mut dyn FnMut()) -> usize {
    struct Paced<'a> {
        step: usize,
        since: usize,
        freed: usize,
        pause: &'a mut dyn FnMut(),
    }
    impl Paced<'_> {
        fn freed(&mut self, nodes: usize) {
            self.freed = self.freed.saturating_add(nodes);
            self.since = self.since.saturating_add(nodes);
            if self.since >= self.step {
                self.since = 0;
                (self.pause)();
            }
        }
    }
    let mut paced = Paced {
        step: step.max(1),
        since: 0,
        freed: 0,
        pause,
    };
    let Index {
        reflects,
        underlyings,
        denoms,
        cells,
        ..
    } = index;
    // `into_iter` frees each node of a map as it is left behind.
    for (_, underlying) in underlyings {
        let Underlying {
            months,
            expiries,
            cells,
            ..
        } = underlying;
        for (_, expiry) in expiries {
            let nodes = expiry
                .contracts
                .len()
                .saturating_add(expiry.months.len())
                .saturating_add(1);
            drop(expiry);
            paced.freed(nodes);
        }
        let nodes = months.len().saturating_add(cells.len()).saturating_add(1);
        drop((months, cells));
        paced.freed(nodes);
    }
    let nodes = denoms.len().saturating_add(cells.len());
    drop((denoms, cells));
    paced.freed(nodes);
    // THE CENSUS READ LAST: its own free (two million entries in two large
    // tables at GDFL's size) after a pause of its own.
    (paced.pause)();
    drop(reflects);
    paced.freed
}

/// `segment`'s index built over every key `slot`'s part holds. An empty
/// index over `slot` when it holds no part (never reached from [`index`],
/// which [`moved_off_lock`] answers first).
fn whole(segment: Segment, slot: &Arc<Slot>) -> Index {
    let mut fresh = Index {
        reflects: Some(Arc::clone(slot)),
        ..Index::default()
    };
    let Some(part) = part_of(slot, segment) else {
        return fresh;
    };
    for key in part.held_keys() {
        if key.segment != segment {
            continue;
        }
        if let Some(held) = part.held(key) {
            fresh.apply(None, &held);
        }
    }
    fresh.generation = part.header().generation;
    fresh
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
    // CASE-FOLDED, as the feed and the underlying are: `fno` is `FNO`
    // (R3-09 repair 1; it was refused while `nifty` was NIFTY).
    let segment = Segment::parse(&segment_word.to_ascii_uppercase()).map_err(|why| {
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
    let mut headers = super::store_route::census_headers_with(census, Some(&behind));
    // HOW CURRENT, on the root answer too (R3-09).
    super::census_cache::freshness_headers(&mut headers, now.freshness());
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
    let mut previous = Answered::Current;
    for segment in SEGMENTS {
        // AN INDEX BEING BUILT IS NAMED, NOT WAITED FOR (R3-09, RA-046).
        let row = match index_or_previous(site, feed, segment, slot, |index| {
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
        }) {
            Ok((row, answered)) => {
                if answered != Answered::Current {
                    previous = answered;
                }
                row
            }
            Err(since_ms) => {
                let (code, body) = warming_answer(&mut headers, feed, segment, since_ms);
                return (code, headers, body);
            }
        };
        if let Some(row) = row {
            if listed > 0 {
                out.push(',');
            }
            out.push_str(&row);
            listed += 1;
        }
    }
    out.push_str("]}");
    super::census_cache::freshness_headers(&mut headers, previous.freshness(now.freshness()));
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
        // HOW CURRENT, on the refusal too, as the roll-up says it (R3-09
        // repair 1): a refusal from the previous read while a re-read runs
        // is marked refreshing, so a reader knows to ask again.
        let mut headers = headers;
        super::census_cache::freshness_headers(&mut headers, now.freshness());
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
    // AN INDEX BEING BUILT IS NAMED, NOT WAITED FOR (R3-09, RA-046): the
    // first request after a restart, or after a new census file, answers
    // "warming" at once instead of waiting the whole build out.
    // The lists answer from the PREVIOUS index while a rebuild runs, said to
    // be previous; the rows (which probe the census by the index's keys) and
    // a first build are answered "warming".
    let warming = |since_ms: u64| {
        let mut headers = headers.clone();
        super::census_cache::freshness_headers(&mut headers, now.freshness());
        let (code, body) = warming_answer(&mut headers, asked.feed, asked.segment, since_ms);
        (code, headers, body)
    };
    let mut answered = Answered::Current;
    let (body, generation) = if asked.rows {
        if let Some(since_ms) = building(site, asked.feed, asked.segment, slot) {
            return warming(since_ms);
        }
        rows_answer(site, &asked, census, slot, state, degraded.as_deref())
    } else {
        match index_or_previous(site, asked.feed, asked.segment, slot, |index| {
            (
                render_answer(index, &asked, state, degraded.as_deref()),
                index.generation,
            )
        }) {
            Ok((out, how)) => {
                answered = how;
                out
            }
            Err(since_ms) => return warming(since_ms),
        }
    };
    // THE GENERATION A `since` IS SPELLED IN, in a header as well as the
    // body, so a client never keys a delta on the vendor-wide sum (WP-56;
    // DB-B56): this part's own, the body's `generation`.
    let mut headers = headers;
    headers.insert(
        axum::http::HeaderName::from_static(super::store_route::CENSUS_PART_GENERATION_HEADER),
        axum::http::HeaderValue::from(generation),
    );
    // HOW CURRENT, on every answer: an answer built from the previous read
    // while a background re-read runs, or from the previous index while a
    // rebuild runs, says so and how old it is (R3-09).
    super::census_cache::freshness_headers(&mut headers, answered.freshness(now.freshness()));
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
                            "the census file cannot answer the appends after that generation \
                             (another file stands at its path, more entries were appended \
                             since than a refresh reads back, or the generation is not one it \
                             published); load the selection afresh"
                        ),
                        part.since_floor(),
                        index.generation
                    ),
                ));
            };
            Some(Changed::of(&changes, asked.segment))
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
        let contracts = under.contracts;
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
                let _ = write!(
                    expiries_json,
                    r#"{{"expiry":{},"weekday":{},"contracts":{},"strikes":{},"future":{},"#,
                    render::json_string(&expiry.to_string()),
                    render::json_string(weekday(*expiry)),
                    chain.contracts.len(),
                    chain.strikes,
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
    fn of(changes: &census::Since<'_>, segment: Segment) -> Self {
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
