//! Corporate-action lineage, wired into the options import and its rescan
//! (D-3331 items 1 and 3, D-3343; findings DB-G71, DB-G72, DB-G77).
//!
//! `crate::gdfl_lineage` (WP-12b) detects a bonus or split from the data
//! alone: on an ex-date the underlying's inferred lot steps by the factor and
//! the vendor renames every open contract to its adjusted strike. This module
//! feeds it, day by day, and writes what it finds:
//!
//! - to `<store>/imports/gdfl.lineage` (D-3331): every `alias` row of
//!   `crate::expiry_alias` once, an `alias` record for a closed stated expiry
//!   the charter's rule settled (first day and contracts as first seen), one
//!   `lineage` record per renamed contract (old key, new key, ex-date, factor,
//!   both inferred lots, and `oi_unadjusted_until` where the vendor carried
//!   the pre-ex OI unscaled), one `stitched` record per store key the
//!   vendor reused for another exchange contract on or after the ex-date,
//!   one `stopped` record per pre-ex key an event retired with no partner,
//!   and one `event` record per detected event naming its kind (lot step,
//!   strike offset with its shift, re-listing), its lots and how many keys
//!   it paired and stopped (D-3355). A re-listing pairs nothing, so before
//!   these two records it left no line at all (DB-B13);
//! - to `<store>/imports/gdfl.lots` (D-3343): a contract's inferred lot when
//!   first seen and when it changes, a reused key starting a new lineage.
//!
//! **The bars are never altered and never split.** The store key has no form
//! for two contracts under one name (D-3331 item 3), so a reused key is
//! recorded `stitched`; `/bars/window.json` reads the ledger and names the
//! first post-ex bar `contract_adjusted`, and the stale-OI span
//! `vendor_oi_unadjusted` (WP-31), instead of serving a +900% change.
//!
//! # The state a detection needs, and how much of it a run has
//!
//! [`crate::gdfl_lineage::detect`] compares an underlying's ex-date
//! contract-days with "the latest pre-ex contract-day of every key the
//! underlying holds that has not expired". A run accumulates that state day
//! by day. A run that starts mid-history, or meets days an earlier run
//! already finished (skipped unread), first reads the open sessions just
//! before the day it imports ([`WARMUP_SESSIONS`], at most [`WARMUP_WALK_DAYS`]
//! calendar days back) and folds them in, read-only: a key that last traded
//! earlier than those sessions is not in the state, so its pair or its
//! `stitched` mark is missed — never invented. A run from the archive's first
//! day (`cli gdfl-dayflags options ... --lineage` over the whole range) holds
//! every key. Measured on the DB-G71 example: NTPC's `12500-PE` last traded
//! four sessions before its 2019-03-19 ex-date, so a run must start by
//! 2019-03-14 to mark it stitched.
//!
//! # The retired keys are the ledger's, not the run's
//!
//! Which pre-ex keys an event retired is read back from the ledger when a
//! run starts ([`LineageRun::load`]): the `lineage`, `stopped` and `stitched`
//! records of every ex-date whose last expiry has not passed. So a run that
//! restarts any number of sessions after an ex-date still marks a reused
//! text `stitched` and restarts its lot, exactly as the run that met the
//! ex-date would have. What a restart can still lack is the PRE-EX state a
//! detection needs (above): that is the warm-up's bound, named there.
//!
//! # Idempotent
//!
//! Every line already in the ledger is read at the start and never written
//! again, and the lot ledger is seeded from its own file, so a re-run over
//! the same days writes nothing. A run that holds MORE of the pre-ex state
//! than the one that first met an ex-date adds the pairs and stops it alone
//! saw and one more `event` record with the larger counts; one that holds
//! less adds nothing.
//!
//! # Cost
//!
//! Per day: O(contracts) to group and update the state, and on an ordinary
//! day `detect` returns after comparing two lots per underlying; on an
//! ex-date O(contracts · log contracts) for that underlying, plus one re-read
//! of each renamed contract's file whose OI crossed unadjusted (18 files in
//! the whole archive, BAJFINANCE 2025-06-16). The ledgers are read once per
//! run, O(their lines), and appended once per day with one `fsync` each.
//! Memory: the latest contract-day of every live key, O(live contracts).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, OptionSide};
use brutex_core::symbol::Symbol;

use crate::expiry_alias::{AliasKind, AliasRecord, Settlement, alias_records};
use crate::gdfl_lineage::{
    ContractDay, CorporateAction, EventKind, EventRecord, LineageRecord, LotLedger, LotRecord,
    OiCarry, StitchedRecord, StoppedRecord, contract_parts, detect, lineage_path, lots_path,
    parse_day,
};
use crate::gdfl_nfo::OptionTicker;
use crate::session::Day;

/// How many open sessions before a run's first options day are folded in,
/// read-only, when the run holds no state for the session just before it.
/// Chosen: a week of the state at the cost of three days' reading.
pub const WARMUP_SESSIONS: usize = 3;

/// How many calendar days back the warm-up looks for those sessions.
pub const WARMUP_WALK_DAYS: u32 = 10;

/// One option file of a day as the lineage reads it: its ticker, its
/// contract-day and its index in the day's listing (for a re-read).
#[derive(Debug, Clone)]
pub struct DayContract {
    /// The decoded ticker.
    pub ticker: OptionTicker,
    /// What its rows showed.
    pub day: ContractDay,
    /// Its index in the day's listing.
    pub file: usize,
}

/// A key as the detector keys it.
type Key = (Day, i64, OptionSide);

/// The key of a contract-day.
const fn key_of(day: &ContractDay) -> Key {
    (day.expiry, day.strike, day.side)
}

/// One event still in force (D-3355): the pre-ex keys it retired, as the
/// ledger and this run together know them. Rebuilt from the ledger at
/// [`LineageRun::load`], so it does not depend on where a run started.
#[derive(Debug, Clone)]
struct Retired {
    ex_date: Day,
    /// The kind the ledger's last `event` record states, or this run's.
    kind: Option<EventKind>,
    /// The inferred lots before and from the ex-date.
    lots: (u64, u64),
    /// Pre-ex key to the key its contract continues under.
    forward: HashMap<Contract, Contract>,
    /// Post-ex key to the pre-ex key renamed onto it (the first, in pre-ex
    /// order, when two were).
    back: HashMap<Contract, Contract>,
    /// Pre-ex keys retired with no partner.
    stopped: HashSet<Contract>,
    /// The last expiry among the retired keys.
    until: Day,
    /// Retired keys whose lot was restarted.
    restarted: HashSet<Contract>,
    /// Whether the ledger holds an `event` record of this ex-date.
    recorded: bool,
}

impl Retired {
    /// An event of `ex_date` that has retired nothing yet.
    fn new(ex_date: Day) -> Self {
        Self {
            ex_date,
            kind: None,
            lots: (0, 0),
            forward: HashMap::new(),
            back: HashMap::new(),
            stopped: HashSet::new(),
            until: ex_date,
            restarted: HashSet::new(),
            recorded: false,
        }
    }

    /// Keeps the entry alive to `key`'s expiry.
    fn reach(&mut self, key: Contract) {
        if let Some(expiry) = expiry_of(key)
            && expiry.days_from_epoch() > self.until.days_from_epoch()
        {
            self.until = expiry;
        }
    }

    /// `old` continues as `new`. Whether the pair is new to the entry. A
    /// pair outranks a stop: the stop said only that no partner was found.
    fn pair(&mut self, old: Contract, new: Contract) -> bool {
        self.reach(old);
        self.stopped.remove(&old);
        self.back.entry(new).or_insert(old);
        match self.forward.entry(old) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(new);
                true
            }
        }
    }

    /// `key` was retired with no partner. Whether that is new to the entry.
    fn stop(&mut self, key: Contract) -> bool {
        self.reach(key);
        !self.forward.contains_key(&key) && self.stopped.insert(key)
    }

    /// Whether the event retired `key`.
    fn holds(&self, key: Contract) -> bool {
        self.forward.contains_key(&key) || self.stopped.contains(&key)
    }

    /// Whether the entry has retired nothing.
    fn is_empty(&self) -> bool {
        self.forward.is_empty() && self.stopped.is_empty()
    }

    /// The `stitched` record of `key`, a retired key traded on the ex-date or
    /// later: where its pre-ex contract went, and which pre-ex contract (if
    /// any) was renamed onto it. Two hash probes.
    fn stitched(&self, underlying: Symbol, key: Contract) -> Option<StitchedRecord> {
        self.holds(key).then(|| StitchedRecord {
            underlying,
            key,
            ex_date: self.ex_date,
            before: self.forward.get(&key).copied(),
            after: self.back.get(&key).copied(),
        })
    }

    /// The kind in force: the recorded one, or what the pairs show for a
    /// ledger written before `event` records existed: a lot step when the
    /// lots differ, the strike offset of any pair when they are equal, a
    /// re-listing when nothing is paired.
    fn kind(&self) -> EventKind {
        if let Some(kind) = self.kind {
            return kind;
        }
        if self.lots.0 != self.lots.1 {
            return EventKind::LotStep;
        }
        self.forward
            .iter()
            .find_map(|(&old, &new)| {
                let shift = contract_parts(old)?.1.checked_sub(contract_parts(new)?.1)?;
                (shift != 0).then_some(EventKind::StrikeOffset { shift })
            })
            .unwrap_or(EventKind::Relisted)
    }

    /// The `event` record of the entry as it stands.
    fn record(&self, underlying: Symbol) -> EventRecord {
        EventRecord {
            underlying,
            ex_date: self.ex_date,
            kind: self.kind(),
            lot_old: self.lots.0,
            lot_new: self.lots.1,
            paired: u64::try_from(self.forward.len()).unwrap_or(u64::MAX),
            stopped: u64::try_from(self.stopped.len()).unwrap_or(u64::MAX),
        }
    }
}

/// One underlying's state.
#[derive(Debug, Clone, Default)]
struct Underlying {
    latest: HashMap<Key, ContractDay>,
    /// The live events by ex-date (days from the epoch), oldest first.
    retired: BTreeMap<u32, Retired>,
}

impl Underlying {
    /// The event of `ex_date`, made when the ledger first names it. One
    /// ordered-map probe over the underlying's live events (a handful).
    fn event(&mut self, ex_date: Day) -> &mut Retired {
        self.retired
            .entry(ex_date.days_from_epoch())
            .or_insert_with(|| Retired::new(ex_date))
    }
}

/// What one day wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Written {
    /// Lines appended to the lineage ledger.
    pub lineage: usize,
    /// Lines appended to the lot ledger.
    pub lots: usize,
    /// Adjustments detected.
    pub actions: usize,
    /// `event` records appended (D-3355): one per newly recorded event.
    pub events: usize,
}

impl Written {
    /// Adds another day's.
    pub const fn absorb(&mut self, other: Self) {
        self.lineage += other.lineage;
        self.lots += other.lots;
        self.actions += other.actions;
        self.events += other.events;
    }
}

/// The lineage of one run: its ledgers, as read at the start, and the state
/// accumulated over the days it has folded in.
#[derive(Debug)]
pub struct LineageRun {
    lineage: PathBuf,
    lots_path: PathBuf,
    seen: HashSet<String>,
    aliased: HashSet<(Option<Symbol>, Day)>,
    lots: LotLedger,
    underlyings: HashMap<Symbol, Underlying>,
    last_day: Option<Day>,
    tables_written: bool,
}

/// The expiry a contract segment names (its first ten bytes).
fn expiry_of(contract: Contract) -> Option<Day> {
    contract.as_str().get(..10).and_then(parse_day)
}

/// One ledger line read back into the run's state, by its first word: an
/// `alias` into the stated days already aliased, and an `event`, `lineage`,
/// `stopped` or `stitched` record into its ex-date's retired keys (D-3355).
/// Any other line, and a line that does not parse as its kind, changes
/// nothing here: the readers that serve the ledger count those.
fn read_back(
    line: &str,
    aliased: &mut HashSet<(Option<Symbol>, Day)>,
    underlyings: &mut HashMap<Symbol, Underlying>,
) {
    /// The retired keys of `underlying`'s event of `ex_date`.
    fn event_of(
        underlyings: &mut HashMap<Symbol, Underlying>,
        underlying: Symbol,
        ex_date: Day,
    ) -> &mut Retired {
        underlyings.entry(underlying).or_default().event(ex_date)
    }
    match line.split(' ').next().unwrap_or_default() {
        "alias" => {
            if let Some(alias) = AliasRecord::parse(line) {
                aliased.insert((alias.underlying, alias.stated));
            }
        }
        "event" => {
            if let Some(record) = EventRecord::parse(line) {
                // The last record of an ex-date is the one in force.
                let entry = event_of(underlyings, record.underlying, record.ex_date);
                entry.kind = Some(record.kind);
                entry.lots = (record.lot_old, record.lot_new);
                entry.recorded = true;
            }
        }
        "lineage" => {
            if let Some(record) = LineageRecord::parse(line) {
                let entry = event_of(underlyings, record.underlying, record.ex_date);
                entry.pair(record.old, record.new);
                if !entry.recorded {
                    entry.lots = (record.lot_old, record.lot_new);
                }
            }
        }
        "stopped" => {
            if let Some(record) = StoppedRecord::parse(line) {
                event_of(underlyings, record.underlying, record.ex_date).stop(record.key);
            }
        }
        // A `stitched` key is a retired key that traded again: a ledger
        // written before `stopped` records existed names its unpaired
        // retired keys only here.
        "stitched" => {
            if let Some(record) = StitchedRecord::parse(line) {
                let entry = event_of(underlyings, record.underlying, record.ex_date);
                match record.before {
                    Some(new) => entry.pair(record.key, new),
                    None => entry.stop(record.key),
                };
                if let Some(old) = record.after {
                    entry.pair(old, record.key);
                }
            }
        }
        _ => {}
    }
}

/// Logs each newly recorded event, once its lines are durable, so the log
/// never names an event the ledger does not hold (D-3355).
fn log_events(events: &[EventRecord]) {
    for record in events {
        let (ex, kind) = (record.ex_date.to_string(), record.kind.as_str());
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::info(super::TARGET, "lineage event recorded")
                .with("underlying", record.underlying.as_str())
                .with("ex", ex.as_str())
                .with("kind", kind)
                .with("shift_paisa", record.kind.shift())
                .with("lot_old", record.lot_old)
                .with("lot_new", record.lot_new)
                .with("paired", record.paired)
                .with("stopped", record.stopped),
        );
    }
}

impl LineageRun {
    /// The run over `store_root`'s ledgers, its lot ledger seeded with the
    /// contracts that have not expired before `from`, and the retired keys of
    /// every event whose last expiry has not passed before `from` rebuilt
    /// from the ledger's `event`, `lineage`, `stopped` and `stitched` records
    /// (D-3355). O(ledger lines), one parse per line by its first word.
    ///
    /// # Errors
    ///
    /// A ledger that exists and cannot be read, named.
    pub fn load(store_root: &Path, from: Day) -> Result<Self, String> {
        let lineage = lineage_path(store_root);
        let lots_path = lots_path(store_root);
        let read = |path: &Path| match std::fs::read(path) {
            Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(why) => Err(format!("{}: {why}", path.display())),
        };
        let mut seen = HashSet::new();
        let mut aliased = HashSet::new();
        let mut underlyings: HashMap<Symbol, Underlying> = HashMap::new();
        for line in read(&lineage)?.lines() {
            read_back(line, &mut aliased, &mut underlyings);
            seen.insert(line.to_owned());
        }
        let mut lots = LotLedger::new();
        for line in read(&lots_path)?.lines() {
            let Some(record) = LotRecord::parse(line) else {
                continue;
            };
            // A lot recorded on or after an ex-date for a key the event
            // retired is the restarted lineage's: the restart is not done
            // again, so a resumed run writes no second first-seen record.
            if let Some(state) = underlyings.get_mut(&record.underlying) {
                for retired in state
                    .retired
                    .range_mut(..=record.from.days_from_epoch())
                    .map(|(_, retired)| retired)
                {
                    if retired.holds(record.contract) {
                        retired.restarted.insert(record.contract);
                    }
                }
            }
            if expiry_of(record.contract)
                .is_none_or(|expiry| expiry.days_from_epoch() >= from.days_from_epoch())
            {
                lots.seed(&record);
            }
        }
        // An event whose last retired key expired before the run's first day
        // can mark nothing: dropped, so the state is O(live events).
        underlyings.retain(|_, state| {
            state
                .retired
                .retain(|_, r| r.until.days_from_epoch() >= from.days_from_epoch());
            !state.retired.is_empty()
        });
        Ok(Self {
            lineage,
            lots_path,
            seen,
            aliased,
            lots,
            underlyings,
            last_day: None,
            tables_written: false,
        })
    }

    /// The last day folded in.
    #[must_use]
    pub const fn last_day(&self) -> Option<Day> {
        self.last_day
    }

    /// Notes `line` for the lineage ledger unless it is already there.
    /// Whether it was noted.
    fn note(&mut self, line: String, out: &mut Vec<String>) -> bool {
        let fresh = self.seen.insert(line.clone());
        if fresh {
            out.push(line);
        }
        fresh
    }

    /// The alias records owed before the first record: every table row once,
    /// and a closed-day-rule settlement the first time it is met.
    fn aliases(&mut self, day: Day, contracts: &[DayContract], out: &mut Vec<String>) {
        if !self.tables_written {
            self.tables_written = true;
            // `alias_records` refuses only a table row that is not a day,
            // which its own tests render: nothing to write is the honest
            // answer if it ever did.
            for record in alias_records().unwrap_or_default() {
                self.aliased.insert((record.underlying, record.stated));
                self.note(record.line(), out);
            }
        }
        let mut closed: BTreeMap<(String, u32), (Symbol, Day, Day, u32)> = BTreeMap::new();
        for c in contracts {
            if c.ticker.settlement == Settlement::ClosedDayRule {
                let slot = closed
                    .entry((
                        c.ticker.underlying.as_str().to_owned(),
                        c.ticker.stated.days_from_epoch(),
                    ))
                    .or_insert((c.ticker.underlying, c.ticker.stated, c.ticker.expiry, 0));
                slot.3 = slot.3.saturating_add(1);
            }
        }
        for (underlying, stated, settled, contracts) in closed.into_values() {
            if self.aliased.insert((Some(underlying), stated)) {
                let record = AliasRecord {
                    underlying: Some(underlying),
                    stated,
                    settled,
                    kind: AliasKind::ClosedDay,
                    first_day: day,
                    contracts,
                };
                self.note(record.line(), out);
            }
        }
    }

    /// Merges a detected `action` into `entry`, the ledger's own record of
    /// its ex-date, and notes what is new: each pair's `lineage` line, a
    /// `stopped` line for every pre-ex key with no partner (DB-B13: a
    /// re-listing retires them all and pairs none), the ex-date's `stitched`
    /// lines, and the `event` line. Returns the event record when its line
    /// is new.
    ///
    /// A run that saw less than the ledger already states (the ex-date was
    /// known, nothing here is new, and its event is recorded) writes
    /// nothing; a re-listing verdict never overrules recorded pairs.
    ///
    /// # Errors
    ///
    /// A re-read the source refused.
    fn merge(
        &mut self,
        action: &CorporateAction,
        entry: &mut Retired,
        file_of: &HashMap<Contract, usize>,
        reread: &mut dyn FnMut(usize) -> Result<Vec<(u32, i64)>, String>,
        lines: &mut Vec<String>,
    ) -> Result<Option<EventRecord>, String> {
        let (underlying, ex_date) = (action.underlying, action.ex_date);
        let had = !entry.is_empty();
        let mut fresh = false;
        for pair in &action.lineage {
            let rows = match (pair.oi, file_of.get(&pair.new)) {
                (OiCarry::Unadjusted, Some(&file)) => reread(file)?,
                _ => Vec::new(),
            };
            self.note(pair.record(&rows).line(), lines);
            fresh |= entry.pair(pair.old, pair.new);
        }
        for &key in &action.unpaired {
            if entry.stop(key) {
                fresh = true;
                let record = StoppedRecord {
                    underlying,
                    key,
                    ex_date,
                };
                self.note(record.line(), lines);
            }
        }
        for &key in &action.stitched {
            if let Some(record) = entry.stitched(underlying, key) {
                self.note(record.line(), lines);
            }
        }
        if (had && !fresh && entry.recorded) || entry.is_empty() {
            return Ok(None);
        }
        let keeps = matches!(action.kind, EventKind::Relisted) && !entry.forward.is_empty();
        if !keeps {
            entry.kind = Some(action.kind);
            entry.lots = (action.lot_before, action.lot_after);
        }
        let record = entry.record(underlying);
        entry.kind = Some(record.kind);
        entry.recorded = true;
        Ok(self.note(record.line(), lines).then_some(record))
    }

    /// Folds in `day`'s option files: detects each underlying's adjustment,
    /// marks every reused key, keeps the lots, and appends what is new to
    /// both ledgers. `reread` reads one file of the day again, by its listing
    /// index, as `(second of day, OI)` rows in file order, for a renamed
    /// contract whose OI crossed unadjusted.
    ///
    /// # Errors
    ///
    /// A re-read the source refused, or a ledger that would not take the
    /// lines, named. What was detected before the failure is in the state;
    /// the lines are written again by the next run over the day, since none
    /// of them was noted as written.
    pub fn day(
        &mut self,
        day: Day,
        contracts: &[DayContract],
        reread: &mut dyn FnMut(usize) -> Result<Vec<(u32, i64)>, String>,
    ) -> Result<Written, String> {
        let mut lines = Vec::new();
        let mut lot_lines = Vec::new();
        let mut events: Vec<EventRecord> = Vec::new();
        let mut written = Written::default();
        self.aliases(day, contracts, &mut lines);
        let mut by: BTreeMap<String, Vec<&DayContract>> = BTreeMap::new();
        for c in contracts {
            by.entry(c.ticker.underlying.as_str().to_owned())
                .or_default()
                .push(c);
        }
        let today = day.days_from_epoch();
        for todays in by.values() {
            let Some(underlying) = todays.first().map(|c| c.ticker.underlying) else {
                continue;
            };
            let mut state = self.underlyings.remove(&underlying).unwrap_or_default();
            state
                .latest
                .retain(|key, _| key.0.days_from_epoch() >= today);
            state
                .retired
                .retain(|_, r| r.until.days_from_epoch() >= today);
            let mut before: Vec<ContractDay> = state.latest.values().copied().collect();
            before.sort_unstable_by_key(key_of);
            let mut after: Vec<ContractDay> = todays.iter().map(|c| c.day).collect();
            after.sort_unstable_by_key(key_of);
            if let Some(action) = detect(underlying, day, &before, &after) {
                written.actions += 1;
                // The ledger's own record of this ex-date, when an earlier
                // run met it: what this run adds is merged into it.
                let mut entry = state
                    .retired
                    .remove(&today)
                    .unwrap_or_else(|| Retired::new(day));
                let file_of: HashMap<Contract, usize> =
                    todays.iter().map(|c| (c.ticker.contract, c.file)).collect();
                let event = self.merge(&action, &mut entry, &file_of, reread, &mut lines)?;
                events.extend(event);
                // Every pre-ex key is retired: renamed or stopped. Its
                // pre-ex contract-day is no longer the state of a live key.
                for c in &before {
                    state.latest.remove(&key_of(c));
                }
                state.retired.insert(today, entry);
            }
            for c in todays {
                let contract = c.ticker.contract;
                // An event read back from the ledger for a LATER day (a run
                // over days before its ex-date) has retired nothing yet: the
                // range stops at today.
                for (_, retired) in state.retired.range_mut(..=today) {
                    if !retired.holds(contract) {
                        continue;
                    }
                    if let Some(record) = retired.stitched(underlying, contract) {
                        let line = record.line();
                        if self.seen.insert(line.clone()) {
                            lines.push(line);
                        }
                    }
                    if retired.restarted.insert(contract) {
                        self.lots.restart(underlying, contract);
                    }
                }
                if let Some(record) = self.lots.observe(underlying, contract, day, c.day.lot) {
                    lot_lines.push(record.line());
                }
                state.latest.insert(key_of(&c.day), c.day);
            }
            self.underlyings.insert(underlying, state);
        }
        self.last_day = Some(day);
        super::append_lines(&self.lineage, &lines)?;
        super::append_lines(&self.lots_path, &lot_lines)?;
        written.lineage = lines.len();
        written.lots = lot_lines.len();
        written.events = events.len();
        log_events(&events);
        Ok(written)
    }
}

/// The open sessions to fold in before `day`, oldest first: at most
/// [`WARMUP_SESSIONS`], within [`WARMUP_WALK_DAYS`] calendar days, and none
/// on or before `after` (the run's last folded day). Empty when `after` is
/// already the session just before `day`. O([`WARMUP_WALK_DAYS`]).
#[must_use]
pub fn warmup_days(day: Day, after: Option<Day>) -> Vec<Day> {
    let mut out = Vec::new();
    let floor = after.map_or(i64::MIN, |d| i64::from(d.days_from_epoch()));
    for back in 1..=WARMUP_WALK_DAYS {
        let at = i64::from(day.days_from_epoch()) - i64::from(back);
        if at <= floor || out.len() >= WARMUP_SESSIONS {
            break;
        }
        let Some(prev) = u32::try_from(at).ok().and_then(|n| Day::from_days(n).ok()) else {
            break;
        };
        if crate::gdfl_cm::session_gate(prev).is_ok() {
            out.push(prev);
        }
    }
    out.reverse();
    out
}
