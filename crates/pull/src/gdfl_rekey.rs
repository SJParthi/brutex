//! The re-key: GDFL option keys the store holds under a name the current
//! decoder disagrees with, moved to the key it gives, as new files, with the
//! old key kept and pointing at the new one. D-3331 (option identity and
//! lineage), D-3316 (census epochs); WP-12c, findings DB-A17 and DB-A18.
//!
//! # The defect this closes
//!
//! The options import that ran from 2018-09-03 to 2020-09-15 was built before
//! D-3331, so it filed every contract under the expiry its NAME states. The
//! decoder at D-3331 files a contract under the day it SETTLED
//! ([`crate::expiry_alias::settle`]) and refuses a strike more than ten times
//! the day's median ([`crate::gdfl_nfo::strike_implausible`]). Measured on the
//! operator's store (state/assurance/NIFTY-OPTIONS.md, 10 Oct 2026):
//!
//! * 13 NIFTY contracts (120 contract-days) are stored under `2023-06-29-*`,
//!   which the decoder settles on 2023-06-28. Resumed on a D-3331 build, the
//!   import would file 606 later days of 9 of them under `2023-06-28-*`, so one
//!   contract would become two series (DB-A17).
//! * 3 NIFTY contracts (`2020-06-25-191250000-CE`, `-191270000-CE`,
//!   `-191300000-CE`, 8 seconds on 2019-05-30) are stored and served although
//!   the decoder refuses them as `StrikeImplausible` (DB-A18).
//!
//! # What the re-key does, and what it never does
//!
//! [`survey`] reads every option key under `bars/gdfl/NSE/FNO/` from its bar
//! files: each month's header and, through the month's time index, the bars of
//! each IST day. No vendor file is read, and nothing is written. [`plan`] then
//! asks the decoder's own two rules of every key, day by day, exactly as the
//! import asks them of a name:
//!
//! 1. **The settled expiry**: [`settle`] of the key's underlying, its stored
//!    expiry (the stated one, for a key the pre-D-3331 import filed) and each
//!    day it holds bars on.
//! 2. **The strike**: against the LOWER median of the underlying's strikes
//!    stored with a bar that day ([`lower_median`]), refused past
//!    [`STRIKE_PLAUSIBLE_MULTIPLE`](crate::gdfl_nfo::STRIKE_PLAUSIBLE_MULTIPLE)
//!    times it. The decoder's ladder is the day's vendor NAMES, which also
//!    holds names that never traded; the store's is the keys with a bar that
//!    day. The two differ only by untraded names, against a bound ten times
//!    the median that real ladders stay under by 5.75 (D-3331 decision 2).
//!
//! A key every day of which is admitted and whose settled expiry is another
//! day is a [`Move`]. A key every day of which is refused is RETIRED
//! ([`RetiredRecord`]). A key with some days refused and some admitted, or a
//! move that would merge two stored keys holding the same day (the decoder's
//! `ContractNamedTwice`), is [`Unresolved`]: named, and left exactly as it is,
//! because no new file can drop a day from an old one.
//!
//! [`apply`] then, in this order, each step idempotent:
//!
//! 1. writes each move's bars under the decoded key through the store's own
//!    writer — NEW month files, never a byte of the old key's (`CLAUDE.md` §3
//!    rule 8). A month already holding exactly those bars is left as it is;
//!    one holding other bars where these would go refuses the move by name;
//! 2. records the new months in the census ([`census_store::record`]);
//! 3. appends a `superseded` line per moved key and a `retired` line per
//!    retired key to `<store>/imports/gdfl.lineage` — the named pointer every
//!    reader of the old key quotes, and the named refusal every reader of a
//!    retired key answers with;
//! 4. installs the census's next FNO epoch without the superseded and retired
//!    keys ([`census_store::withdraw`]), the epoch it replaces kept whole.
//!
//! A second run writes nothing: the months are already present, the census
//! rows exact, the lines already there and the keys already withdrawn.
//!
//! # Cost
//!
//! [`survey`] is O(files · days per month): one header read per month file
//! and, per IST day it spans, one time-index lookup (at most one entry read
//! with a `.tix`, a bisection without one). [`plan`] is O(key-days) plus one
//! sort per underlying-day ladder. [`apply`] is O(bars moved) and one census
//! cycle per partition. None of it is on a request path: `cli gdfl-rekey`
//! only. No bench times it. Measured once, 10 Oct 2026, on shadows of the
//! operator's store (WP-12c), wall clock on a disk other jobs shared: the
//! whole GDFL FNO tree (213,365 keys, 367,189 month files) surveyed and
//! planned in 14 min 26 s; NIFTY alone (11,803 keys, 22,983 files) in 44 to
//! 49 s, and its apply (32 month files written, 16 ledger lines, one census
//! cycle and one epoch) inside that; a second apply in 4.8 s. A single run,
//! not a distribution.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Expiry, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, Row as _};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use crate::expiry_alias::{AliasKind, settle};
use crate::gdfl_lineage::{
    RetiredRecord, RetiredWhy, SupersededRecord, contract_parts, lineage_path, option_key,
};
use crate::gdfl_nfo::{lower_median, strike_implausible};
use crate::ingest::census_store;
use crate::manifest::{Closes, Entry, EntryKey, Held};
use crate::session::{Day, IstMoment};

/// The telemetry target every line here is filed under.
const TARGET: &str = "pull.rekey";

/// The one rung GDFL option bars are stored at (D-2806), and so the one the
/// re-key reads and writes. A key holding any other rung is [`Unresolved`].
pub const RUNG: Timeframe = Timeframe::SECOND_1;

/// Paths a survey names one by one before it only counts them.
pub const NAMED: usize = 50;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Seconds in a civil day.
const DAY_SECS: i64 = 86_400;

/// IST is UTC + 05:30, in seconds.
const IST_SECS: i64 = 19_800;

/// The id the store stamps into every bar file of `symbol`: the low half of
/// its 64-bit FNV-1a, exactly as `crate::ingest` writes it and every reader
/// checks it.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the symbol id is the low half of a 64-bit FNV-1a, exactly as \
              `ingest::identify` derives it; the narrowing IS the identity"
)]
fn symbol_id(symbol: Symbol) -> u32 {
    brutex_core::universe::fnv1a(symbol.as_str()) as u32
}

/// The IST day an instant falls on.
fn ist_day(ts_micros: i64) -> Option<Day> {
    IstMoment::from_epoch_secs(ts_micros.div_euclid(MICROS))
        .ok()
        .map(IstMoment::day)
}

/// 00:00 IST of `day`, in UTC microseconds.
fn ist_midnight_micros(day: Day) -> i64 {
    (i64::from(day.days_from_epoch()) * DAY_SECS - IST_SECS) * MICROS
}

/// The store path parts of one month of `underlying`'s option `contract` at
/// [`RUNG`].
fn month_parts(underlying: &Symbol, contract: Contract, month: YearMonth) -> PathParts<'_> {
    PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: underlying.as_str(),
        contract: Some(contract),
        timeframe: RUNG,
        month,
        file: FileKind::Bars,
    }
}

// ═══ The ledger, as the re-key and its readers see it ═══════════════════════

/// The keys `<store>/imports/gdfl.lineage` withdraws: every `superseded` and
/// `retired` record (D-3331; WP-12c). What `cli census-recount` must not
/// count back, what the census's next epoch leaves out, and what a re-run of
/// the re-key finds already written.
#[derive(Debug, Clone, Default)]
pub struct Withdrawn {
    superseded: HashMap<(Symbol, Contract), SupersededRecord>,
    retired: HashMap<(Symbol, Contract), RetiredRecord>,
    lines: HashSet<String>,
    /// Whether the file's last line has no newline: a torn append.
    torn_tail: bool,
}

impl Withdrawn {
    /// The records of a ledger's text. Lines of the ledger's other kinds
    /// (`alias`, `lineage`, `stitched`) and lines a crash marked torn are not
    /// the re-key's and are passed over.
    ///
    /// # Errors
    ///
    /// A `superseded` or `retired` line that does not parse, one key both
    /// superseded and retired, or one key superseded or retired twice with
    /// different records, named by line: a ledger the re-key cannot read whole
    /// is refused, never read as the part that parsed.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut out = Self {
            superseded: HashMap::with_capacity(64),
            retired: HashMap::with_capacity(16),
            lines: HashSet::with_capacity(80),
            torn_tail: !text.is_empty() && !text.ends_with('\n'),
        };
        for (number, line) in text.lines().enumerate() {
            let at = number.saturating_add(1);
            if line.ends_with(" (torn)") {
                continue;
            }
            if line.starts_with("superseded ") {
                let record = SupersededRecord::parse(line).ok_or_else(|| {
                    format!("line {at} does not parse as a `superseded` record: {line:?}")
                })?;
                // A KEY MOVES ONCE. A second line naming the same key with
                // different fields was a silent replacement — the later target
                // won — in a ledger this function promises to read whole or
                // refuse (cloud-attack-law-20261016). An identical repeat (a
                // crash re-write) is the same record and is kept.
                if let Some(held) = out
                    .superseded
                    .insert((record.underlying, record.old), record)
                    && held != record
                {
                    return Err(format!(
                        "line {at}: {} is superseded twice, to {} and to {}; a key moves once",
                        option_key(record.underlying, record.old),
                        held.new.as_str(),
                        record.new.as_str()
                    ));
                }
            } else if line.starts_with("retired ") {
                let record = RetiredRecord::parse(line).ok_or_else(|| {
                    format!("line {at} does not parse as a `retired` record: {line:?}")
                })?;
                if let Some(held) = out.retired.insert((record.underlying, record.key), record)
                    && held != record
                {
                    return Err(format!(
                        "line {at}: {} is retired twice with different records; a key is \
                         retired once",
                        option_key(record.underlying, record.key)
                    ));
                }
            } else {
                continue;
            }
            let _ = out.lines.insert(line.to_owned());
        }
        if let Some((underlying, contract)) = out
            .superseded
            .keys()
            .find(|key| out.retired.contains_key(*key))
        {
            return Err(format!(
                "{} is both superseded and retired; a key is one or the other",
                option_key(*underlying, *contract)
            ));
        }
        Ok(out)
    }

    /// Whether the census row `key` belongs to a withdrawn key. One or two
    /// hash probes.
    #[must_use]
    pub fn holds(&self, key: &EntryKey) -> bool {
        let Some(contract) = key.contract else {
            return false;
        };
        if key.exchange != Exchange::Nse || key.segment != Segment::Fno {
            return false;
        }
        let at = (key.symbol, contract);
        self.superseded.contains_key(&at) || self.retired.contains_key(&at)
    }

    /// The `superseded` record of `underlying`'s `contract`, if any.
    #[must_use]
    pub fn superseded(&self, underlying: Symbol, contract: Contract) -> Option<&SupersededRecord> {
        self.superseded.get(&(underlying, contract))
    }

    /// The `retired` record of `underlying`'s `contract`, if any.
    #[must_use]
    pub fn retired(&self, underlying: Symbol, contract: Contract) -> Option<&RetiredRecord> {
        self.retired.get(&(underlying, contract))
    }

    /// How many keys are superseded and how many retired.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        (self.superseded.len(), self.retired.len())
    }

    /// Whether this exact line is already in the ledger.
    fn has_line(&self, line: &str) -> bool {
        self.lines.contains(line)
    }
}

/// The withdrawn keys of the store at `root`: its `imports/gdfl.lineage`, or
/// none when the ledger does not exist.
///
/// # Errors
///
/// A ledger that exists and cannot be read, or that [`Withdrawn::parse`]
/// refuses, naming the file.
pub fn withdrawn(root: &Path) -> Result<Withdrawn, String> {
    let path = lineage_path(root);
    match fs::read_to_string(&path) {
        Ok(text) => Withdrawn::parse(&text).map_err(|why| format!("{}: {why}", path.display())),
        Err(absent) if absent.kind() == ErrorKind::NotFound => Ok(Withdrawn::default()),
        Err(why) => Err(format!("{} could not be read: {why}", path.display())),
    }
}

// ═══ The survey: what the store holds, read off the bar files ═══════════════

/// One stored month file of a key, from its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Month {
    /// The month.
    pub month: YearMonth,
    /// Committed bars.
    pub rows: u64,
    /// The first bar's stamp, UTC microseconds.
    pub first_ts_micros: i64,
    /// The last bar's stamp, UTC microseconds.
    pub last_ts_micros: i64,
}

/// What the store holds for one GDFL option key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    /// The underlying.
    pub underlying: Symbol,
    /// The stored contract segment.
    pub contract: Contract,
    /// The stored expiry: the stated one, for a key the pre-D-3331 import
    /// filed.
    pub expiry: Day,
    /// The strike, paisa.
    pub strike: i64,
    /// Call or put.
    pub side: OptionSide,
    /// Its month files holding at least one bar, oldest first.
    pub months: Vec<Month>,
    /// Bars per IST day.
    pub days: BTreeMap<Day, u64>,
}

impl Stored {
    /// Every bar the key holds.
    #[must_use]
    pub fn bars(&self) -> u64 {
        self.months.iter().map(|month| month.rows).sum()
    }

    /// The census text of the key.
    #[must_use]
    pub fn key(&self) -> String {
        option_key(self.underlying, self.contract)
    }
}

/// What [`survey`] found under `bars/gdfl/NSE/FNO/`.
#[derive(Debug, Clone, Default)]
pub struct Survey {
    /// Every option key with at least one bar, in underlying and contract
    /// order.
    pub keys: Vec<Stored>,
    /// Month files read.
    pub files: u64,
    /// Month files that would not read, counted; no plan is applied while
    /// there is one, because a ladder missing a file is not the day's ladder.
    pub unreadable: u64,
    /// The first [`NAMED`] of them, with why.
    pub unreadable_named: Vec<(PathBuf, String)>,
    /// Directory entries that are not part of an option's store path.
    pub strays: u64,
    /// The first [`NAMED`] of them.
    pub strays_named: Vec<PathBuf>,
    /// Keys holding a rung other than [`RUNG`], which the re-key does not
    /// move; each is also [`Unresolved`] in the plan.
    pub other_rungs: Vec<String>,
}

impl Survey {
    fn stray(&mut self, path: PathBuf) {
        self.strays = self.strays.saturating_add(1);
        if self.strays_named.len() < NAMED {
            self.strays_named.push(path);
        }
    }

    fn note_unreadable(&mut self, path: PathBuf, why: String) {
        self.unreadable = self.unreadable.saturating_add(1);
        if self.unreadable_named.len() < NAMED {
            self.unreadable_named.push((path, why));
        }
    }
}

/// One directory's entries by name, in name order (a `BTreeMap`, so two
/// surveys of one store read keys in one order). An absent directory is an
/// empty one.
fn listed(dir: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(why) => return Err(format!("{} could not be listed: {why}", dir.display())),
    };
    let mut out = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{} could not be listed: {why}", dir.display()))?;
        let _ = out.insert(
            entry.file_name().to_string_lossy().into_owned(),
            entry.path(),
        );
    }
    Ok(out)
}

/// The month a `YYYY-MM` stem is, in the store's own spelling.
fn month_named(stem: &str) -> Option<YearMonth> {
    let (year, month) = stem.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    let month = YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()?;
    (month.to_string() == stem).then_some(month)
}

/// Bars per IST day of one open month file: for each day its header spans,
/// the rows from that day's first bar to the next day's, by time-index
/// lookups. O(days spanned) lookups.
///
/// # Errors
///
/// A lookup the store refuses.
pub fn day_counts(file: &BarFile) -> Result<Vec<(Day, u64)>, StoreError> {
    let header = file.header();
    let mut out = Vec::with_capacity(31);
    if header.n_valid == 0 {
        return Ok(out);
    }
    let (Some(first), Some(last)) = (
        ist_day(header.first_ts_micros),
        ist_day(header.last_ts_micros),
    ) else {
        return Ok(out);
    };
    let mut at = 0_u64;
    let mut day = first;
    loop {
        let end = if day >= last {
            header.n_valid
        } else {
            let next = Day::from_days(day.days_from_epoch().saturating_add(1)).unwrap_or(last);
            file.first_at_or_after(ist_midnight_micros(next))?
        };
        if end > at {
            out.push((day, end - at));
        }
        at = end;
        if day >= last {
            return Ok(out);
        }
        match Day::from_days(day.days_from_epoch().saturating_add(1)) {
            Ok(next) => day = next,
            Err(_) => return Ok(out),
        }
    }
}

/// Reads every option key under `bars/gdfl/NSE/FNO/` (or only `only`'s
/// underlyings) from its bar files: each month's header and its bars per IST
/// day. Read-only: every month is opened through
/// [`BarFile::open_existing`], under a SHARED lock, and nothing is created.
///
/// # Errors
///
/// A directory that exists and cannot be listed. A month file that will not
/// open is counted in [`Survey::unreadable`], not returned.
pub fn survey(root: &Path, only: Option<&BTreeSet<Symbol>>) -> Result<Survey, String> {
    let mut found = Survey::default();
    let base = root
        .join(store::path::STORE_ROOT)
        .join(Vendor::Gdfl.as_str())
        .join(Exchange::Nse.as_str())
        .join(Segment::Fno.as_str());
    for (name, path) in listed(&base)? {
        let Some(underlying) = Symbol::new(&name)
            .ok()
            .filter(|symbol| symbol.as_str() == name)
        else {
            found.stray(path);
            continue;
        };
        if only.is_some_and(|wanted| wanted.get(&underlying).is_none()) {
            continue;
        }
        for (name, path) in listed(&path)? {
            let parsed = Contract::parse(&name)
                .filter(|contract| contract.as_str() == name)
                .and_then(|contract| contract_parts(contract).map(|parts| (contract, parts)));
            let Some((contract, (expiry, strike, side))) = parsed else {
                found.stray(path);
                continue;
            };
            let mut key = Stored {
                underlying,
                contract,
                expiry,
                strike,
                side,
                months: Vec::new(),
                days: BTreeMap::new(),
            };
            for (rung, rung_path) in listed(&path)? {
                if rung != RUNG.as_str() {
                    found.other_rungs.push(format!("{} {rung}", key.key()));
                    continue;
                }
                read_rung(root, &mut key, &rung_path, &mut found)?;
            }
            if !key.months.is_empty() {
                found.keys.push(key);
            }
        }
    }
    Ok(found)
}

/// Every month file of one key's [`RUNG`] directory, into `key`.
fn read_rung(root: &Path, key: &mut Stored, dir: &Path, found: &mut Survey) -> Result<(), String> {
    for (name, path) in listed(dir)? {
        let Some(stem) = name.strip_suffix(FileKind::Bars.extension()) else {
            if !FileKind::ALL
                .iter()
                .any(|kind| name.ends_with(kind.extension()))
            {
                found.stray(path);
            }
            continue;
        };
        let Some(month) = month_named(stem) else {
            found.stray(path);
            continue;
        };
        found.files = found.files.saturating_add(1);
        let opened = StorePath::new(month_parts(&key.underlying, key.contract, month))
            .map_err(|why| why.to_string())
            .and_then(|at| {
                BarFile::open_existing(root, at, symbol_id(key.underlying))
                    .map_err(|why| why.to_string())
            });
        let file = match opened {
            Ok(file) => file,
            Err(why) => {
                found.note_unreadable(path, why);
                continue;
            }
        };
        let header = file.header();
        if header.n_valid == 0 {
            continue;
        }
        match day_counts(&file) {
            Ok(days) => {
                for (day, bars) in days {
                    let held = key.days.entry(day).or_insert(0);
                    *held = held.saturating_add(bars);
                }
                key.months.push(Month {
                    month,
                    rows: header.n_valid,
                    first_ts_micros: header.first_ts_micros,
                    last_ts_micros: header.last_ts_micros,
                });
            }
            Err(why) => found.note_unreadable(path, why.to_string()),
        }
    }
    Ok(())
}

// ═══ The plan: the decoder's two rules, asked of every stored key ═══════════

/// One stored key the decoder files under another: its bars move there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    /// The underlying both keys name.
    pub underlying: Symbol,
    /// The stored key.
    pub from: Contract,
    /// The key the decoder gives.
    pub to: Contract,
    /// Why the expiry moved.
    pub kind: AliasKind,
    /// The stored key's month files, oldest first.
    pub months: Vec<YearMonth>,
    /// Its bars.
    pub bars: u64,
}

/// A key the re-key leaves exactly as it is, and why: named, never silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The census text of the key.
    pub key: String,
    /// Why nothing was done.
    pub why: String,
}

/// What [`plan`] decided for every surveyed key.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Keys judged.
    pub keys: u64,
    /// Keys the decoder files where they are, every day admitted.
    pub kept: u64,
    /// Keys to move, in key order.
    pub moves: Vec<Move>,
    /// Keys to retire, in key order.
    pub retired: Vec<RetiredRecord>,
    /// Keys left as they are, with why.
    pub unresolved: Vec<Unresolved>,
}

impl Plan {
    /// Whether the plan would change anything.
    #[must_use]
    pub fn changes(&self) -> bool {
        !self.moves.is_empty() || !self.retired.is_empty()
    }
}

/// One refused day of one key.
#[derive(Debug, Clone)]
struct Refused {
    day: Day,
    why: RetiredWhy,
    median: Option<i64>,
    text: String,
}

/// The verdict on one key.
enum Verdict {
    Keep,
    Move(Contract, AliasKind),
    Retire(RetiredRecord),
    Unresolved(String),
}

/// The decoder's two rules, asked of one stored key on every day it holds.
fn judge(key: &Stored, medians: &HashMap<(Symbol, Day), i64>) -> Verdict {
    let under = key.underlying.as_str();
    let mut settled: Option<(Day, Option<AliasKind>)> = None;
    let mut refused: Vec<Refused> = Vec::with_capacity(key.days.len());
    for &day in key.days.keys() {
        // THE DECODER'S ORDER: the expiry is settled first (inside
        // `decode_ticker`), the strike checked after (`check_strike`).
        let settlement = match settle(under, key.expiry, Some(day)) {
            Ok(settlement) => settlement,
            Err(why) => {
                refused.push(Refused {
                    day,
                    why: RetiredWhy::ExpiryUnsettled,
                    median: None,
                    text: format!("{day}: expiry unsettled: {why}"),
                });
                continue;
            }
        };
        if let Some(&median) = medians.get(&(key.underlying, day))
            && strike_implausible(key.strike, median)
        {
            refused.push(Refused {
                day,
                why: RetiredWhy::StrikeImplausible,
                median: Some(median),
                text: format!("{day}: strike implausible against the day's median {median} paisa"),
            });
            continue;
        }
        let this = (settlement.expiry, settlement.settlement.kind());
        match settled {
            None => settled = Some(this),
            Some(seen) if seen == this => {}
            Some((seen, _)) => {
                return Verdict::Unresolved(format!(
                    "the decoder settles its days on two expiries, {seen} and {}; \
                     refused rather than split",
                    this.0
                ));
            }
        }
    }
    let held = u64::try_from(key.days.len()).unwrap_or(u64::MAX);
    if let Some(first) = refused.first() {
        if refused.len() == key.days.len() {
            return Verdict::Retire(RetiredRecord {
                underlying: key.underlying,
                key: key.contract,
                why: first.why,
                day: first.day,
                median: first.median,
                days: held,
                bars: key.bars(),
            });
        }
        let named: Vec<&str> = refused
            .iter()
            .take(NAMED)
            .map(|one| one.text.as_str())
            .collect();
        return Verdict::Unresolved(format!(
            "the decoder refuses {} of its {held} day(s) and admits the rest ({}); a new \
             file cannot drop a day from an old one, so this needs the month regenerated \
             (WP-80), and nothing was done",
            refused.len(),
            named.join("; ")
        ));
    }
    match settled {
        Some((expiry, Some(kind))) if expiry != key.expiry => {
            match settled_contract(expiry, key.strike, key.side) {
                Some(to) => Verdict::Move(to, kind),
                None => Verdict::Unresolved(format!(
                    "the settled expiry {expiry} does not render as a store contract segment"
                )),
            }
        }
        _ => Verdict::Keep,
    }
}

/// The key the current decoder files `underlying`'s stored option `contract`
/// under, and why, when that is ANOTHER key: its stored expiry, read as the
/// vendor's stated one, settles elsewhere ([`settle`], D-3331 decision 1).
/// `None` for a key the decoder files where it is, a future, a segment that
/// does not read back, and an expiry the settlement refuses (the re-key
/// judges that one day by day). What a reader says of a stored key before
/// the re-key has moved it. O(1): one settlement, a bounded walk over
/// compile-time tables; UNVERIFIED as a measurement.
#[must_use]
pub fn decoded_key(underlying: Symbol, contract: Contract) -> Option<(Contract, AliasKind)> {
    let (expiry, strike, side) = contract_parts(contract)?;
    let settled = settle(underlying.as_str(), expiry, None).ok()?;
    let kind = settled.settlement.kind()?;
    (settled.expiry != expiry)
        .then(|| settled_contract(settled.expiry, strike, side).map(|to| (to, kind)))
        .flatten()
}

/// The contract segment of an option settled on `expiry`.
fn settled_contract(expiry: Day, strike: i64, side: OptionSide) -> Option<Contract> {
    let expiry = Expiry::new(expiry.year(), expiry.month(), expiry.day()).ok()?;
    Contract::of(Kind::Option {
        expiry,
        strike: Paisa::from_raw(strike),
        side,
    })
}

/// Decides what to do with every surveyed key (module doc). Pure: reads
/// nothing and writes nothing.
///
/// A key the ledger already names superseded is left out of the day's
/// ladders, because the key it continues as is in them with the same strike
/// and days; it is still judged, so a re-run plans the same move and finds
/// it done.
#[must_use]
pub fn plan(survey: &Survey, ledger: &Withdrawn) -> Plan {
    let mut ladders: HashMap<(Symbol, Day), Vec<i64>> =
        HashMap::with_capacity(survey.keys.len().saturating_mul(4));
    for key in &survey.keys {
        if ledger.superseded(key.underlying, key.contract).is_some() {
            continue;
        }
        for &day in key.days.keys() {
            ladders
                .entry((key.underlying, day))
                .or_insert_with(|| Vec::with_capacity(64))
                .push(key.strike);
        }
    }
    let mut medians: HashMap<(Symbol, Day), i64> = HashMap::with_capacity(ladders.len());
    for (at, mut strikes) in ladders {
        if let Some(median) = lower_median(&mut strikes) {
            let _ = medians.insert(at, median);
        }
    }
    let mut out = Plan {
        keys: u64::try_from(survey.keys.len()).unwrap_or(u64::MAX),
        ..Plan::default()
    };
    for key in &survey.other_rungs {
        out.unresolved.push(Unresolved {
            key: key.clone(),
            why: format!(
                "holds a rung other than {}, which the re-key does not move",
                RUNG.as_str()
            ),
        });
    }
    let held: HashMap<(Symbol, Contract), &Stored> = survey
        .keys
        .iter()
        .map(|key| ((key.underlying, key.contract), key))
        .collect();
    for key in &survey.keys {
        match judge(key, &medians) {
            Verdict::Keep => out.kept = out.kept.saturating_add(1),
            Verdict::Move(to, kind) => out.moves.push(Move {
                underlying: key.underlying,
                from: key.contract,
                to,
                kind,
                months: key.months.iter().map(|month| month.month).collect(),
                bars: key.bars(),
            }),
            Verdict::Retire(record) => out.retired.push(record),
            Verdict::Unresolved(why) => out.unresolved.push(Unresolved {
                key: key.key(),
                why,
            }),
        }
    }
    named_twice(&mut out, &held, ledger);
    out
}

/// Moves that would merge two stored keys holding one day — the decoder's
/// `ContractNamedTwice`, which refuses both names that day — and moves into a
/// key the plan retires or leaves unresolved are taken out of the plan and
/// named.
///
/// A move the ledger already records into the same key is a re-run: its
/// days are the target's own copy of them, so it is not checked against
/// them again.
fn named_twice(out: &mut Plan, held: &HashMap<(Symbol, Contract), &Stored>, ledger: &Withdrawn) {
    let blocked: HashSet<String> = out
        .retired
        .iter()
        .map(|record| option_key(record.underlying, record.key))
        .chain(out.unresolved.iter().map(|one| one.key.clone()))
        .collect();
    let mut into: BTreeMap<(Symbol, Contract), Vec<usize>> = BTreeMap::new();
    for (at, one) in out.moves.iter().enumerate() {
        into.entry((one.underlying, one.to)).or_default().push(at);
    }
    let mut refused: BTreeMap<usize, String> = BTreeMap::new();
    for ((underlying, to), sources) in &into {
        let target = option_key(*underlying, *to);
        if blocked.contains(target.as_str()) {
            for &at in sources {
                let _ = refused.insert(
                    at,
                    format!("its decoded key {target} is itself refused or unresolved"),
                );
            }
            continue;
        }
        // Every day each source holds, against every other source and the
        // target the store already holds.
        let mut seen: HashMap<Day, String> = HashMap::with_capacity(64);
        if let Some(stored) = held.get(&(*underlying, *to)) {
            for &day in stored.days.keys() {
                let _ = seen.insert(day, target.clone());
            }
        }
        for &at in sources {
            let Some(one) = out.moves.get(at) else {
                continue;
            };
            let Some(stored) = held.get(&(one.underlying, one.from)) else {
                continue;
            };
            if ledger
                .superseded(one.underlying, one.from)
                .is_some_and(|done| done.new == one.to)
            {
                continue;
            }
            let name = option_key(one.underlying, one.from);
            for &day in stored.days.keys() {
                if let Some(other) = seen.insert(day, name.clone()) {
                    let why = format!(
                        "it and {other} both hold {day} and the decoder files both under \
                         {target}: two names of one contract on one day, which the decoder \
                         refuses (ContractNamedTwice)"
                    );
                    for &also in sources {
                        let _ = refused.entry(also).or_insert_with(|| why.clone());
                    }
                }
            }
        }
    }
    for (at, why) in refused.into_iter().rev() {
        if at < out.moves.len() {
            let gone = out.moves.remove(at);
            out.unresolved.push(Unresolved {
                key: option_key(gone.underlying, gone.from),
                why,
            });
        }
    }
}

// ═══ Applying a plan ════════════════════════════════════════════════════════

/// Why a key leaves the census: the two kinds of withdrawn key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetiredOr {
    /// It continues under another key.
    Superseded,
    /// The decoder refuses every day of it.
    Retired,
}

/// What [`apply`] did.
#[derive(Debug, Clone, Default)]
pub struct Applied {
    /// One record per moved key, as written to the ledger (or found there).
    pub moved: Vec<SupersededRecord>,
    /// Month files written under a decoded key.
    pub months_written: u64,
    /// Month files that already held exactly the bars to be written.
    pub months_already: u64,
    /// Moves refused when their target was read, with why. Nothing of a
    /// refused move was written.
    pub refused: Vec<Unresolved>,
    /// The census cycles that recorded the new months.
    pub census: Vec<census_store::Cycle>,
    /// Ledger lines appended.
    pub lines_written: u64,
    /// The census epoch installed without the withdrawn keys, if one was.
    pub withdrawal: Option<census_store::Compaction>,
}

/// How one target month is to be written.
enum Write {
    /// No bar there yet, or the bars follow the ones there: appended.
    Append,
    /// Exactly these bars are there already: nothing written.
    Already,
}

/// The bars of one month of one stored key, read whole through the verified
/// read path.
fn read_month(
    root: &Path,
    underlying: Symbol,
    contract: Contract,
    month: YearMonth,
) -> Result<Vec<Bar>, String> {
    let at = StorePath::new(month_parts(&underlying, contract, month))
        .map_err(|why| format!("{} {month}: {why}", option_key(underlying, contract)))?;
    let shown = at.to_path_buf(root);
    let file = BarFile::open_existing(root, at, symbol_id(underlying))
        .map_err(|why| format!("{}: {why}", shown.display()))?;
    let n = file.header().n_valid;
    let mut bars = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for index in 0..n {
        bars.push(
            file.read_record(index)
                .map_err(|why| format!("{} record {index}: {why}", shown.display()))?,
        );
    }
    Ok(bars)
}

/// What the target month already holds decides how `bars` go there: an
/// absent or empty month, or one whose last bar precedes them, is appended
/// to; one holding exactly them at their place is left; anything else is
/// refused, naming the month.
fn target_state(
    root: &Path,
    underlying: Symbol,
    to: Contract,
    month: YearMonth,
    bars: &[Bar],
) -> Result<Write, String> {
    let at = StorePath::new(month_parts(&underlying, to, month))
        .map_err(|why| format!("{} {month}: {why}", option_key(underlying, to)))?;
    let shown = at.to_path_buf(root);
    let file = match BarFile::open_existing(root, at, symbol_id(underlying)) {
        Ok(file) => file,
        Err(StoreError::Missing { .. }) => return Ok(Write::Append),
        Err(why) => return Err(format!("{}: {why}", shown.display())),
    };
    let header = file.header();
    let (Some(first), Some(last)) = (bars.first(), bars.last()) else {
        return Ok(Write::Already);
    };
    if header.n_valid == 0 || first.ts_micros > header.last_ts_micros {
        return Ok(Write::Append);
    }
    let start = file
        .first_at_or_after(first.ts_micros)
        .map_err(|why| format!("{}: {why}", shown.display()))?;
    let len = u64::try_from(bars.len()).unwrap_or(u64::MAX);
    let fits = start
        .checked_add(len)
        .is_some_and(|end| end <= header.n_valid);
    if fits {
        let mut same = true;
        for (offset, bar) in (0_u64..).zip(bars) {
            let held = file
                .read_record(start.saturating_add(offset))
                .map_err(|why| format!("{}: {why}", shown.display()))?;
            if held != *bar {
                same = false;
                break;
            }
        }
        if same {
            return Ok(Write::Already);
        }
    }
    Err(format!(
        "{} already holds bars from {} to {} that these {} bar(s) from {} to {} would \
         precede or contradict; a month is never rewritten in place, so moving them \
         needs the month regenerated (WP-80): run the re-key before the import resumes",
        shown.display(),
        header.first_ts_micros,
        header.last_ts_micros,
        bars.len(),
        first.ts_micros,
        last.ts_micros
    ))
}

/// The census row of a month as its open writer handle now holds it: rows
/// and both stamps from the committed header, the closes from its first and
/// last record — the four facts the ingest records after an append.
fn census_row(
    file: &BarFile,
    underlying: Symbol,
    to: Contract,
    month: YearMonth,
) -> Result<Held, String> {
    let header = file.header();
    let first = file.read_record(0).map_err(|why| why.to_string())?;
    let last = file
        .read_record(header.n_valid.saturating_sub(1))
        .map_err(|why| why.to_string())?;
    let closes = Closes::known(first.close, last.close).map_err(|why| why.to_string())?;
    Ok(Held::new(
        Entry {
            key: EntryKey {
                contract: Some(to),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: underlying,
                timeframe: RUNG,
                month,
            },
            rows: header.n_valid,
            first_ts_micros: header.first_ts_micros,
            last_ts_micros: header.last_ts_micros,
        },
        closes,
    ))
}

/// One target key's bars by month, merged, and each source's digest.
type Merged = (BTreeMap<YearMonth, Vec<Bar>>, Vec<[u8; 32]>);

/// One target key's moves: every source month read, merged by month in time
/// order, checked against what the target holds, then written.
struct Group<'a> {
    underlying: Symbol,
    to: Contract,
    /// The stored keys moving here, in key order.
    sources: BTreeMap<Contract, &'a Move>,
}

/// What one group wrote.
struct Done {
    records: Vec<SupersededRecord>,
    rows: Vec<Held>,
    written: u64,
    already: u64,
}

impl Group<'_> {
    /// Every month's bars of every source, merged in time order: each
    /// source-month is one run of increasing stamps, the runs are laid down
    /// by their first stamp, and the result must strictly increase. Two
    /// sources of one target never share a day (`named_twice`), so a stamp
    /// held twice is a fault, never merged. Also the digest of each source's
    /// bars, in the order they are written.
    fn read(&self, root: &Path) -> Result<Merged, String> {
        let target = option_key(self.underlying, self.to);
        let mut runs: BTreeMap<YearMonth, BTreeMap<i64, Vec<Bar>>> = BTreeMap::new();
        let mut digests: Vec<[u8; 32]> = Vec::with_capacity(self.sources.len());
        for (from, one) in &self.sources {
            let mut hasher = brutex_core::blake3::Hasher::new();
            for &month in &one.months {
                let bars = read_month(root, self.underlying, *from, month)?;
                let mut image = Vec::with_capacity(bars.len().saturating_mul(Bar::LEN));
                for bar in &bars {
                    bar.write_into(&mut image);
                }
                hasher.update(&image);
                let Some(first) = bars.first().map(|bar| bar.ts_micros) else {
                    continue;
                };
                if runs.entry(month).or_default().insert(first, bars).is_some() {
                    return Err(format!(
                        "{target} {month}: two of the keys moving here start on one second"
                    ));
                }
            }
            digests.push(hasher.finalize());
        }
        let mut by_month: BTreeMap<YearMonth, Vec<Bar>> = BTreeMap::new();
        for (month, laid) in runs {
            let merged: Vec<Bar> = laid.into_values().flatten().collect();
            if merged
                .windows(2)
                .any(|pair| matches!(pair, [a, b] if a.ts_micros >= b.ts_micros))
            {
                return Err(format!(
                    "{target} {month}: the keys moving here hold interleaved or repeated \
                     seconds; refused rather than merged"
                ));
            }
            let _ = by_month.insert(month, merged);
        }
        Ok((by_month, digests))
    }

    /// Reads, checks and writes the group; nothing is written unless every
    /// month checked.
    fn run(&self, root: &Path) -> Result<Done, String> {
        let (by_month, digests) = self.read(root)?;
        let mut checked: Vec<(YearMonth, Vec<Bar>, Write)> = Vec::with_capacity(by_month.len());
        for (month, bars) in by_month {
            let write = target_state(root, self.underlying, self.to, month, &bars)?;
            checked.push((month, bars, write));
        }
        // THEN WRITTEN, month by month, through the store's own writer.
        let mut done = Done {
            records: Vec::with_capacity(self.sources.len()),
            rows: Vec::with_capacity(checked.len()),
            written: 0,
            already: 0,
        };
        let id = symbol_id(self.underlying);
        for (month, bars, write) in &checked {
            let at = StorePath::new(month_parts(&self.underlying, self.to, *month))
                .map_err(|why| why.to_string())?;
            let shown = at.to_path_buf(root);
            let named = |why: &dyn std::fmt::Display| format!("{}: {why}", shown.display());
            let file = match write {
                Write::Append => {
                    let mut file =
                        BarFile::open_or_create(root, at, id).map_err(|why| named(&why))?;
                    match file.append(bars).map_err(|why| named(&why))? {
                        Appended::Committed { .. } => {
                            done.written = done.written.saturating_add(1);
                        }
                        Appended::AlreadyPresent { .. } => {
                            done.already = done.already.saturating_add(1);
                        }
                    }
                    file
                }
                Write::Already => {
                    done.already = done.already.saturating_add(1);
                    BarFile::open_existing(root, at, id).map_err(|why| named(&why))?
                }
            };
            done.rows.push(
                census_row(&file, self.underlying, self.to, *month).map_err(|why| named(&why))?,
            );
        }
        for ((from, one), digest) in self.sources.iter().zip(digests) {
            done.records.push(SupersededRecord {
                underlying: self.underlying,
                old: *from,
                new: self.to,
                kind: one.kind,
                months: u64::try_from(one.months.len()).unwrap_or(u64::MAX),
                bars: one.bars,
                digest,
            });
        }
        Ok(done)
    }
}

/// Appends `lines` to the ledger at `path` in one write, synced; a torn last
/// line is closed with the `(torn)` mark first (D-2814's rule).
fn append_lines(path: &Path, lines: &[String], torn_tail: bool) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    }
    let mut text = String::with_capacity(lines.len().saturating_mul(160).saturating_add(8));
    if torn_tail {
        text.push_str(" (torn)\n");
    }
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|why| format!("{} could not be opened to append: {why}", path.display()))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{} could not be appended to: {why}", path.display()))
}

/// Applies `plan` to the store at `root` (module doc, steps 1 to 4).
///
/// # Errors
///
/// Refused before anything is written: a survey that could not read a month
/// file, and a GDFL census that is not partitioned. After that, a census
/// cycle, the ledger or the withdrawal that refuses, naming what was already
/// done — every step is idempotent, so a re-run finishes it. A MOVE whose
/// target refuses is not an error: it is named in [`Applied::refused`] and
/// nothing of it is written.
pub fn apply(
    root: &Path,
    survey: &Survey,
    plan: &Plan,
    ledger: &Withdrawn,
) -> Result<Applied, String> {
    preflight(root, survey)?;
    let mut out = Applied::default();
    let rows = move_all(root, plan, &mut out);
    if !rows.is_empty() {
        out.census = census_store::record(root, Vendor::Gdfl, &rows).map_err(|why| {
            format!(
                "{why} ({} month file(s) were written under decoded keys and are not yet \
                 counted; no ledger line was written. Run again to finish)",
                out.months_written
            )
        })?;
    }
    let lines: Vec<String> = out
        .moved
        .iter()
        .map(SupersededRecord::line)
        .chain(plan.retired.iter().map(RetiredRecord::line))
        .filter(|line| !ledger.has_line(line))
        .collect();
    append_lines(&lineage_path(root), &lines, ledger.torn_tail).map_err(|why| {
        format!("{why} (the moved months are written and counted; run again to finish)")
    })?;
    out.lines_written = u64::try_from(lines.len()).unwrap_or(u64::MAX);
    out.withdrawal = withdraw_all(root, &out, plan, ledger).map_err(|why| {
        format!(
            "{why} (the moved months are written and counted and the ledger holds every \
             line; run again to finish)"
        )
    })?;
    note_applied(&out, plan);
    Ok(out)
}

/// The refusals [`apply`] makes before anything is written: a survey that
/// missed a month file, and a census a key cannot be withdrawn from.
fn preflight(root: &Path, survey: &Survey) -> Result<(), String> {
    if survey.unreadable > 0 {
        return Err(format!(
            "the survey could not read {} month file(s), so a day's ladder may be missing \
             strikes; nothing was written",
            survey.unreadable
        ));
    }
    match census_store::locate(root, Vendor::Gdfl)? {
        census_store::Location::Partitioned(_) => Ok(()),
        census_store::Location::Single(path) => Err(format!(
            "the gdfl census is a single file ({}); the re-key withdraws keys by installing \
             the FNO partition's next epoch, which only a migrated census has. Run `cli \
             census-migrate gdfl` first (D-3316, WP-80 step 1); nothing was written",
            path.display()
        )),
    }
}

/// Step 1: every move, grouped by the key it moves to, written; the census
/// rows of what was written are returned, and a group that refuses is named
/// in [`Applied::refused`] with nothing of it written.
fn move_all(root: &Path, plan: &Plan, out: &mut Applied) -> Vec<Held> {
    let mut groups: BTreeMap<(Symbol, Contract), Group<'_>> = BTreeMap::new();
    for one in &plan.moves {
        let _ = groups
            .entry((one.underlying, one.to))
            .or_insert_with(|| Group {
                underlying: one.underlying,
                to: one.to,
                sources: BTreeMap::new(),
            })
            .sources
            .insert(one.from, one);
    }
    let mut rows: Vec<Held> = Vec::with_capacity(groups.len().saturating_mul(4));
    for group in groups.values() {
        match group.run(root) {
            Ok(done) => {
                out.months_written = out.months_written.saturating_add(done.written);
                out.months_already = out.months_already.saturating_add(done.already);
                rows.extend(done.rows);
                out.moved.extend(done.records);
            }
            Err(why) => {
                for one in group.sources.values() {
                    note_refused(&option_key(one.underlying, one.from), &why);
                    out.refused.push(Unresolved {
                        key: option_key(one.underlying, one.from),
                        why: why.clone(),
                    });
                }
            }
        }
    }
    rows
}

/// Step 4: the census stops listing every key the ledger now withdraws —
/// this run's and any earlier run's, so a run cut short before this step is
/// finished by the next.
fn withdraw_all(
    root: &Path,
    out: &Applied,
    plan: &Plan,
    ledger: &Withdrawn,
) -> Result<Option<census_store::Compaction>, String> {
    let mut gone: HashMap<(Symbol, Contract), RetiredOr> = HashMap::with_capacity(
        out.moved
            .len()
            .saturating_add(plan.retired.len())
            .saturating_add(ledger.superseded.len())
            .saturating_add(ledger.retired.len()),
    );
    for at in ledger.superseded.keys().copied().chain(
        out.moved
            .iter()
            .map(|record| (record.underlying, record.old)),
    ) {
        let _ = gone.insert(at, RetiredOr::Superseded);
    }
    for at in ledger.retired.keys().copied().chain(
        plan.retired
            .iter()
            .map(|record| (record.underlying, record.key)),
    ) {
        let _ = gone.insert(at, RetiredOr::Retired);
    }
    let withdrawn = |key: &EntryKey| {
        key.exchange == Exchange::Nse
            && key
                .contract
                .is_some_and(|contract| gone.contains_key(&(key.symbol, contract)))
    };
    census_store::withdraw(root, Vendor::Gdfl, Segment::Fno, &withdrawn)
}

/// One refused move, at `Warn`.
fn note_refused(key: &str, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "move refused")
            .with("key", telemetry::Value::Str(key))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// The run's outcome, at `Info`, or `Warn` when anything was left undone.
fn note_applied(out: &Applied, plan: &Plan) {
    let level = if out.refused.is_empty() && plan.unresolved.is_empty() {
        telemetry::Level::Info
    } else {
        telemetry::Level::Warn
    };
    let count = |n: usize| telemetry::Value::Uint(u64::try_from(n).unwrap_or(u64::MAX));
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(level, TARGET, "re-key applied")
            .with("moved", count(out.moved.len()))
            .with("retired", count(plan.retired.len()))
            .with("refused", count(out.refused.len()))
            .with("unresolved", count(plan.unresolved.len()))
            .with("months_written", telemetry::Value::Uint(out.months_written))
            .with("months_already", telemetry::Value::Uint(out.months_already))
            .with("lines_written", telemetry::Value::Uint(out.lines_written))
            .with(
                "epoch",
                out.withdrawal
                    .as_ref()
                    .map_or(telemetry::Value::Int(-1), |done| {
                        telemetry::Value::Uint(u64::from(done.to_epoch))
                    }),
            ),
    );
}

#[cfg(test)]
#[path = "gdfl_rekey_tests.rs"]
mod tests;
