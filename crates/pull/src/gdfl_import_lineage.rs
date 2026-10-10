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
//!   the pre-ex OI unscaled), and one `stitched` record per store key the
//!   vendor reused for another exchange contract on or after the ex-date;
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
//! # Idempotent
//!
//! Every line already in the ledger is read at the start and never written
//! again, and the lot ledger is seeded from its own file, so a re-run over
//! the same days writes nothing.
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
    ContractDay, CorporateAction, LotLedger, LotRecord, OiCarry, detect, lineage_path, lots_path,
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

/// One adjustment still in force: the action, the keys it retired, the last
/// expiry among them, and the retired keys whose lot was restarted.
#[derive(Debug, Clone)]
struct Retired {
    action: CorporateAction,
    keys: HashSet<Contract>,
    until: Day,
    restarted: HashSet<Contract>,
}

/// One underlying's state.
#[derive(Debug, Clone, Default)]
struct Underlying {
    latest: HashMap<Key, ContractDay>,
    retired: Vec<Retired>,
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
}

impl Written {
    /// Adds another day's.
    pub const fn absorb(&mut self, other: Self) {
        self.lineage += other.lineage;
        self.lots += other.lots;
        self.actions += other.actions;
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

impl LineageRun {
    /// The run over `store_root`'s ledgers, its lot ledger seeded with the
    /// contracts that have not expired before `from`.
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
        for line in read(&lineage)?.lines() {
            if let Some(alias) = AliasRecord::parse(line) {
                aliased.insert((alias.underlying, alias.stated));
            }
            seen.insert(line.to_owned());
        }
        let mut lots = LotLedger::new();
        for line in read(&lots_path)?.lines() {
            if let Some(record) = LotRecord::parse(line)
                && expiry_of(record.contract)
                    .is_none_or(|expiry| expiry.days_from_epoch() >= from.days_from_epoch())
            {
                lots.seed(&record);
            }
        }
        Ok(Self {
            lineage,
            lots_path,
            seen,
            aliased,
            lots,
            underlyings: HashMap::new(),
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
    fn note(&mut self, line: String, out: &mut Vec<String>) {
        if self.seen.insert(line.clone()) {
            out.push(line);
        }
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
            state.retired.retain(|r| r.until.days_from_epoch() >= today);
            let mut before: Vec<ContractDay> = state.latest.values().copied().collect();
            before.sort_unstable_by_key(key_of);
            let mut after: Vec<ContractDay> = todays.iter().map(|c| c.day).collect();
            after.sort_unstable_by_key(key_of);
            if let Some(action) = detect(underlying, day, &before, &after) {
                written.actions += 1;
                let file_of: HashMap<Contract, usize> =
                    todays.iter().map(|c| (c.ticker.contract, c.file)).collect();
                for pair in &action.lineage {
                    let rows = match (pair.oi, file_of.get(&pair.new)) {
                        (OiCarry::Unadjusted, Some(&file)) => reread(file)?,
                        _ => Vec::new(),
                    };
                    self.note(pair.record(&rows).line(), &mut lines);
                }
                for record in action.stitched_records() {
                    self.note(record.line(), &mut lines);
                }
                // Every pre-ex key is retired: renamed or stopped. Its
                // pre-ex contract-day is no longer the state of a live key.
                for c in &before {
                    state.latest.remove(&key_of(c));
                }
                let until = before.iter().map(|c| c.expiry).max().unwrap_or(day);
                state.retired.push(Retired {
                    keys: action.retired(),
                    action,
                    until,
                    restarted: HashSet::new(),
                });
            }
            for c in todays {
                let contract = c.ticker.contract;
                for retired in &mut state.retired {
                    if !retired.keys.contains(&contract) {
                        continue;
                    }
                    if retired.action.ex_date.days_from_epoch() < today
                        && let Some(record) = retired.action.stitched_record(contract)
                    {
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
