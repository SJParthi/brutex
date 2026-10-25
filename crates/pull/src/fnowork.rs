//! WHICH EXPIRED CONTRACT-MONTHS ARE MISSING — [`crate::work`]'s question,
//! asked about contracts instead of series.
//!
//! # The gap this module closes
//!
//! The spot backfill drives itself: `crate::work::gaps` answers *expected
//! minus held*, `api::autopilot` walks the answer oldest first, and re-running
//! fetches nothing when nothing is missing. Expired derivatives had every part
//! of that except the question. [`crate::chain`] discovers a month's contracts
//! and [`crate::rolling`] builds Dhan's cross product, but nothing asked which
//! of the contracts they name are **already on disk** — so the only way to
//! drive them was one HTTP request per operator click, and "the entire expired
//! F&O history" was not a thing anybody could start.
//!
//! This module is that question, and nothing else.
//!
//! # Why the resume point is the store and not a ledger
//!
//! A completion marker — *"underlying U, month M, feed F: done"* — is the
//! obvious design and it is the one this crate already refuses for spot.
//! `api::autopilot`'s header states the reason: a cursor is a second answer to
//! *where am I*, and the copy that is wrong is the one that skips a month in
//! silence. So there is no ledger here either. A contract-month is missing iff
//! the manifest does not hold it, which is one hash probe against a counter
//! that is maintained on write.
//!
//! The price is re-running **discovery** on a month whose bars are all held.
//! That is one expiries call plus one contracts call per expiry — units, not
//! thousands — against a bars saving of every contract in the month. Paying it
//! keeps the resume point derivable from the store after a kill, a crash or a
//! machine change, which a marker file cannot promise.
//!
//! # Discovery is keyed by EXPIRY month; bars are keyed by BAR month
//!
//! These are not the same month and conflating them is the defect this module
//! is shaped to avoid. `chain::month(NIFTY, 2026, 1)` answers *which contracts
//! expired in January 2026*. A weekly that expired 2026-01-08 traded only
//! inside January, but a monthly that expired 2026-01-29 was listed for
//! roughly three months and has bars in 2025-11 and 2025-12 as well.
//!
//! So [`cells`] takes the bar months **from the caller** rather than assuming
//! the expiry's own. A driver that passes one month gets the expiry month and
//! is honest about covering only that; a driver that passes three gets the
//! contract's likely whole life. Neither is decided here, because deciding it
//! here would hide the choice from the operator — the shape `CLAUDE.md` §6
//! rejects for sweep depth, one layer down.
//!
//! # Cost
//!
//! | operation | cost |
//! |---|---|
//! | [`ladder`] | O(underlyings × months), one allocation reserved up front |
//! | [`cells`] | O(contracts × months), likewise |
//! | [`gaps`] | O(cells), **one hash probe each** |
//!
//! Nothing here lists a directory, opens a bar file or grows with the size of
//! the store — `docs/07-o1-architecture.md` law 3. The held set is the
//! manifest's own key set, so a probe here is the probe `crate::work` already
//! makes for spot.
//!
//! Neither is a timing. Every claim above is a count of operations, and no
//! bench in this crate measures this module yet.

use std::collections::HashSet;

use crate::session::Day;
use brutex_core::error::InstrumentError;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use store::path::{Timeframe, YearMonth};

use crate::fno::Found;
use crate::manifest::EntryKey;

/// The exchange every cell here is filed under.
///
/// `CLAUDE.md` §1 fixes the engine surface at NSE, so this is a constant rather
/// than a field: a parameter would offer a caller a choice the charter does not
/// grant, and an `Exchange::Bse` cell would render a store path for data this
/// build does not pull.
const VENUE: Exchange = Exchange::Nse;

/// One (underlying, month) pair the discovery has to be asked about.
///
/// Deliberately **not** [`crate::fno::Ask`]: that one carries a year and a
/// month as separate integers because the vendor's query string does, and it
/// carries an `expiry` field that is empty on the first of the two calls. This
/// is the *work item*, addressed the way the store addresses a month, and it
/// converts into that one at the call site rather than standing in for it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ask {
    /// The underlying's plain symbol — `BANKNIFTY`, never a contract name.
    pub underlying: String,
    /// The month whose expiries are being asked for.
    pub month: YearMonth,
}

/// Every discovery ask a set of underlyings over a set of months implies.
///
/// The order is underlying-major and month-ascending **within** each
/// underlying, which is the order a backfill must run in: `crate::ingest`
/// refuses a batch spanning two months and `store::file::BarFile::append`
/// refuses a batch whose overlap with what is held is not a suffix of it, so a
/// later month fetched first permanently blocks the earlier days of that
/// month's file. The caller is free to sort differently and this ordering is
/// what it gets if it does not.
///
/// ```
/// # use pull::fnowork::{Ask, ladder};
/// # use store::path::YearMonth;
/// let jan = YearMonth::new(2026, 1)?;
/// let feb = YearMonth::new(2026, 2)?;
///
/// let work = ladder(&["BANKNIFTY", "NIFTY"], &[jan, feb]);
///
/// assert_eq!(work.len(), 4);
/// assert_eq!(work[0], Ask { underlying: "BANKNIFTY".to_owned(), month: jan });
/// assert_eq!(work[1], Ask { underlying: "BANKNIFTY".to_owned(), month: feb });
/// assert_eq!(work[2], Ask { underlying: "NIFTY".to_owned(), month: jan });
/// # Ok::<(), store::path::PathError>(())
/// ```
#[must_use]
pub fn ladder(underlyings: &[&str], months: &[YearMonth]) -> Vec<Ask> {
    // Reserved from a bound known before the loop — `docs/07-o1-architecture.md`
    // law 2 — so the vector never grows mid-pass.
    let mut out = Vec::with_capacity(underlyings.len().saturating_mul(months.len()));
    for underlying in underlyings {
        for month in months {
            out.push(Ask {
                underlying: (*underlying).to_owned(),
                month: *month,
            });
        }
    }
    out
}

/// One expired contract's month — the unit an F&O backfill is measured in.
///
/// It carries the manifest key rather than the fields that build it, because
/// the key is what the probe in [`gaps`] hashes and a second spelling of
/// "which slice of data" is a second thing to keep in step. The vendor's own
/// name rides along because that is what goes back on the wire to ask for the
/// bars, and recovering it from the key would mean re-rendering a name the
/// discovery already returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCell {
    /// Exactly the tuple the manifest counts and the store addresses.
    pub key: EntryKey,
    /// The vendor's own contract name, as discovered.
    pub vendor_symbol: String,
    /// When this contract expired.
    ///
    /// Carried because completeness needs it: a contract's last possible bar in
    /// a month is the earliest of the month's end, the operator's window end,
    /// and the day it expired. Without the expiry, every contract's final month
    /// reads short forever — it stops trading mid-month and no vendor will ever
    /// send the rest.
    pub expiry: brutex_core::instrument::Expiry,
}

/// Why a discovered contract could not be turned into a cell.
///
/// It is an error and not a skip for the reason [`crate::chain::Chain`]
/// keeps its `unreadable` list: a contract dropped in silence is a month that
/// reads complete and is not, which is the one class of failure that corrupts
/// a backtest years later without ever announcing itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellError {
    /// The underlying was not a name this store can file under.
    Underlying {
        /// The name as the vendor gave it.
        name: String,
        /// What the symbol reader said about it.
        why: InstrumentError,
    },
}

impl core::fmt::Display for CellError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Underlying { ref name, ref why } => write!(
                f,
                "the discovered contract's underlying {name:?} is not a symbol this \
                 store can file under ({why}), so no cell was built for it and its \
                 bars were not asked for"
            ),
        }
    }
}

impl core::error::Error for CellError {}

/// What one month's discovery produced, and what it could not.
///
/// Both halves, always. A caller that reports only `cells` reports a month as
/// covered while some of its contracts were never asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovered {
    /// One cell per (contract, bar month).
    pub cells: Vec<ContractCell>,
    /// Every contract that produced no cell, and why.
    pub refused: Vec<CellError>,
}

/// Turn one month's discovered contracts into the cells their bars land in.
///
/// `months` is the set of **bar** months to cover, and it is the caller's
/// choice — see this module's header for why the expiry's own month is not
/// assumed.
///
/// ```
/// # use pull::fnowork::cells;
/// # use pull::fno::Found;
/// # use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
/// # use brutex_core::price::Paisa;
/// # use store::path::{Timeframe, YearMonth};
/// let expiry = Expiry::new(2026, 1, 29)?;
/// let kind = Kind::Option { expiry, strike: Paisa::from_raw(5_800_000), side: OptionSide::Call };
/// let found = Found {
///     vendor_symbol: "BANKNIFTY26JAN58000CE".to_owned(),
///     underlying: "BANKNIFTY".to_owned(),
///     contract: Contract::of(kind).expect("an option has a contract segment"),
///     expiry,
///     // THE STRIKE AND THE SIDE, KEPT rather than re-parsed out of the
///     // rendered contract — which is a path segment, not a pair of values to
///     // compute with. `pull::pricing` needs both.
///     option: Some((Paisa::from_raw(5_800_000), OptionSide::Call)),
/// };
///
/// let jan = YearMonth::new(2026, 1)?;
/// let out = cells(&[found], &[jan], Timeframe::MINUTE_1);
///
/// assert_eq!(out.cells.len(), 1);
/// assert!(out.refused.is_empty());
/// assert_eq!(out.cells[0].vendor_symbol, "BANKNIFTY26JAN58000CE");
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[must_use]
pub fn cells(found: &[Found], months: &[YearMonth], timeframe: Timeframe) -> Discovered {
    let mut out = Discovered {
        cells: Vec::with_capacity(found.len().saturating_mul(months.len())),
        refused: Vec::new(),
    };
    for contract in found {
        // THE SYMBOL IS READ ONCE PER CONTRACT, not once per month. A name this
        // store cannot file under is refused for every month at once, so one
        // bad underlying produces one error rather than one per month — which
        // is what makes the refused list readable.
        let symbol = match Symbol::new(&contract.underlying) {
            Ok(symbol) => symbol,
            Err(why) => {
                out.refused.push(CellError::Underlying {
                    name: contract.underlying.clone(),
                    why,
                });
                continue;
            }
        };
        for month in months {
            out.cells.push(ContractCell {
                expiry: contract.expiry,
                key: EntryKey {
                    contract: Some(contract.contract),
                    exchange: VENUE,
                    // A CONTRACT IS ALWAYS F&O. `Segment::Fno` covers futures
                    // and options both — `pull::vendor` says so where it builds
                    // its segment token table — so there is no branch here on
                    // which of the two this is.
                    segment: Segment::Fno,
                    symbol,
                    timeframe,
                    month: *month,
                },
                vendor_symbol: contract.vendor_symbol.clone(),
            });
        }
    }
    out
}

/// What a round of discovery left to fetch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Work {
    /// The cells whose bars are not on disk, in the order they were offered.
    pub missing: Vec<ContractCell>,
    /// How many of the offered cells were already held.
    pub held: usize,
}

impl Work {
    /// Whether there is nothing left to fetch.
    ///
    /// Distinct from `missing.is_empty()` in what it is *used for*: a caller
    /// asking this is asking whether to stop, and reading the vector directly
    /// invites the same question being answered two ways in two places.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }

    /// How many cells were offered in total.
    #[must_use]
    pub const fn offered(&self) -> usize {
        self.missing.len().saturating_add(self.held)
    }
}

/// One contract-month that still owes bars, and the day to resume it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resume {
    /// The cell, exactly as discovery offered it.
    pub cell: ContractCell,
    /// The first day still owed. Never before the window's own start.
    pub from: Day,
    /// The last day this cell can EVER owe a bar for — see [`owed`].
    pub through: Day,
}

/// What a round of discovery still owes, as POSITIONS rather than flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Owed {
    /// The cells that are short, each with the day to resume from.
    pub resume: Vec<Resume>,
    /// How many offered cells owe nothing more.
    pub complete: usize,
    /// Complete cells whose month, window and expiry have no common day.
    /// These require no fetch, but do not prove that any history is held.
    pub outside_window: usize,
}

impl Owed {
    /// Whether there is nothing left to fetch.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.resume.is_empty()
    }

    /// How many cells were offered in total.
    #[must_use]
    pub const fn offered(&self) -> usize {
        self.resume.len().saturating_add(self.complete)
    }
}

/// What is still owed on each contract-month — a POSITION, not a boolean.
///
/// # Why the boolean version of this was withdrawn
///
/// [`gaps_by`] asks `held: Fn(&EntryKey) -> bool`, and a first attempt at an
/// incremental F&O gate wired it to `Manifest::entry(key).is_some()`.
///
/// [`EntryKey`] is `(contract, exchange, segment, symbol, timeframe, month)` —
/// **the window is not in it** — and `Entry::check` refuses only `rows == 0`.
/// So ONE BAR made a contract-month "held", and a month whose pull died on day
/// five reported complete for all thirty-one. Worse, §8's append-only rule
/// meant a re-run could not repair it: the file already held the prefix, so the
/// wider batch was refused.
///
/// It replaced a loud, correct 502 with a silent success — which is exactly the
/// "fallback that hides a failure" §4 bans. The gate was withdrawn rather than
/// patched, because a boolean cannot express half a month no matter how it is
/// wired.
///
/// # What replaces it
///
/// The shape `api::autopilot::next_window` has used for spot since it was
/// written: probe the last held TIMESTAMP and resume from the day after. The
/// manifest already carries it as `Entry::last_ts_micros`, so this costs the
/// same single probe the boolean did and answers a strictly harder question.
///
/// `held` returns that stamp for a key, or `None` for a key the manifest does
/// not carry. It must be O(1) — a hash probe, not a search — and that is a
/// contract with the caller this function cannot enforce, so it is stated
/// rather than assumed.
///
/// # The last day a CONTRACT can owe is not the last day of the month
///
/// This is the one place F&O differs from spot, and getting it wrong costs
/// every run rather than one. A contract's owed-through is the earliest of:
///
/// * the month's own last day,
/// * the operator's window end,
/// * **the day the contract expired**.
///
/// A spot series trades every session of every month, so its month end is its
/// answer. A contract stops trading mid-month and no vendor will ever send the
/// rest — so judged against the month end, every contract's final month reads
/// short forever and is refetched on every run, returning empty every time.
///
/// A cell whose whole span falls outside the window — an expiry before the
/// window opens, a month after it closes — owes nothing and is counted
/// complete rather than asked for.
///
/// # Cost
///
/// **O(cells)**: one probe and a fixed number of integer comparisons each.
/// Never O(store). Nothing here opens a file or lists a directory.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn owed<F>(cells: &[ContractCell], window: (Day, Day), held: F) -> Owed
where
    F: Fn(&EntryKey) -> Option<i64>,
{
    let mut out = Owed {
        resume: Vec::with_capacity(cells.len()),
        complete: 0,
        outside_window: 0,
    };
    for cell in cells {
        // NOTHING FETCHABLE. Not a gap — a cell the window and the expiry
        // between them leave no day in.
        let Some((first, through)) = span_of(cell, window) else {
            out.complete = out.complete.saturating_add(1);
            out.outside_window = out.outside_window.saturating_add(1);
            continue;
        };
        // ONE PROBE. Nothing here opens a file or lists a directory.
        let from = match held(&cell.key).and_then(day_of) {
            // Nothing held for this key in this month: the whole span is owed.
            None => first,
            Some(day) if day < through => {
                // The last representable day has no successor. Counted as
                // complete rather than asked for again, because there is no
                // day after it to ask for.
                let Ok(next) = day.succ() else {
                    out.complete = out.complete.saturating_add(1);
                    continue;
                };
                next
            }
            // Held through the last day this cell can ever owe.
            Some(_) => {
                out.complete = out.complete.saturating_add(1);
                continue;
            }
        };
        out.resume.push(Resume {
            cell: cell.clone(),
            // A resume point before the window's start is clamped UP to it,
            // never down: asking earlier than the operator did would write
            // days they did not request, and asking later would lose them.
            from: from.max(first),
            through,
        });
    }
    out
}

/// The days of `window` this cell can hold bars for, or `None` for none.
///
/// Clamped below by the month's first day and the window's start, above by the
/// month's last day, the window's end, and the expiry. See [`owed`] on why the
/// expiry belongs in that list.
fn span_of(cell: &ContractCell, window: (Day, Day)) -> Option<(Day, Day)> {
    let (from, to) = window;
    let month = cell.key.month;
    let opens = Day::new(month.year(), month.month(), 1).ok()?;
    let expiry = Day::new(cell.expiry.year(), cell.expiry.month(), cell.expiry.day()).ok()?;
    let first = opens.max(from);
    let through = opens.end_of_month().min(to).min(expiry);
    if through < first {
        None
    } else {
        Some((first, through))
    }
}

/// The IST day a stored stamp falls in.
///
/// `api::autopilot::day_of` is the same three lines for spot. It is not shared
/// because the arrow points the other way: `api` depends on `pull`, so `pull`
/// cannot reach it, and CLAUDE.md §5's graph is acyclic by rule.
fn day_of(ts_micros: i64) -> Option<Day> {
    crate::session::IstMoment::from_epoch_secs(ts_micros.div_euclid(1_000_000))
        .ok()
        .map(crate::session::IstMoment::day)
}

/// Gap = discovered − held, asking a PROBE rather than a prepared set.
///
/// # Why this exists beside [`gaps`]
///
/// [`gaps`] takes a `HashSet`, which a caller has to build first. The manifest
/// is not a set — it is a counter file with its own `entry` probe — and turning
/// it into one would mean walking every committed entry to answer a question
/// about the handful of cells actually discovered. That is O(store) to avoid
/// O(cells), which is backwards, and `docs/07-o1-architecture.md` law 3 says so:
/// never scan to answer a question.
///
/// `held` is called once per cell and must itself be O(1) — a hash probe, not a
/// search. That is a contract with the caller this function cannot enforce, and
/// it is stated rather than assumed.
///
/// # Cost
///
/// O(cells), one call to `held` each. Nothing here allocates beyond the missing
/// cells it returns.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn gaps_by<F: Fn(&EntryKey) -> bool>(discovered: &[ContractCell], held: F) -> Work {
    let mut work = Work {
        missing: Vec::with_capacity(discovered.len()),
        held: 0,
    };
    for cell in discovered {
        if held(&cell.key) {
            work.held = work.held.saturating_add(1);
        } else {
            work.missing.push(cell.clone());
        }
    }
    work
}

/// Gap = discovered − held, one hash probe per cell.
///
/// `held` is the manifest's own key set, so this asks the counter rather than
/// the file tree — `docs/07-o1-architecture.md` law 3, and the same probe
/// `crate::work::gaps` makes for spot.
///
/// ```
/// # use std::collections::HashSet;
/// # use pull::fnowork::{ContractCell, gaps};
/// # use pull::manifest::EntryKey;
/// # use brutex_core::instrument::{Contract, Exchange, Expiry, Kind, OptionSide, Segment};
/// # use brutex_core::price::Paisa;
/// # use brutex_core::symbol::Symbol;
/// # use store::path::{Timeframe, YearMonth};
/// # let expiry = Expiry::new(2026, 1, 29)?;
/// # let kind = Kind::Option { expiry, strike: Paisa::from_raw(5_800_000), side: OptionSide::Call };
/// # let key = EntryKey {
/// #     contract: Contract::of(kind),
/// #     exchange: Exchange::Nse,
/// #     segment: Segment::Fno,
/// #     symbol: Symbol::new("BANKNIFTY")?,
/// #     timeframe: Timeframe::MINUTE_1,
/// #     month: YearMonth::new(2026, 1)?,
/// # };
/// let cell = ContractCell {
///     key,
///     vendor_symbol: "BANKNIFTY26JAN58000CE".to_owned(),
///     // WHEN IT STOPPED TRADING. `gaps` does not read it — a set membership
///     // test cannot — but `owed` does, and it is the field that keeps a
///     // contract's final month from reading short on every run forever.
///     expiry,
/// };
///
/// // Nothing held: it is missing.
/// let empty = HashSet::new();
/// assert_eq!(gaps(&[cell.clone()], &empty).missing.len(), 1);
///
/// // Held: a re-run asks for nothing at all.
/// let mut held = HashSet::new();
/// held.insert(key);
/// let work = gaps(&[cell], &held);
/// assert!(work.is_complete());
/// assert_eq!(work.held, 1);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[must_use]
pub fn gaps<S: core::hash::BuildHasher>(
    discovered: &[ContractCell],
    held: &HashSet<EntryKey, S>,
) -> Work {
    let mut work = Work {
        missing: Vec::with_capacity(discovered.len()),
        held: 0,
    };
    for cell in discovered {
        if held.contains(&cell.key) {
            work.held = work.held.saturating_add(1);
        } else {
            work.missing.push(cell.clone());
        }
    }
    work
}

// ─────── futures day candles through a token: stored, never swept ───────
//
// R4-28, CON-12 and FUT-C1; D-7186. A future is filed as one contract keyed
// (underlying, expiry) through `Contract::of(Kind::Future)`, under
// `bars/<vendor>/NSE/FNO/<UNDERLYING>/<expiry>-FUT/1day/`, with its open
// interest. The vendor's token is only the ADDRESS: which contract it names is
// read from the dated master snapshots, and a token the newest snapshot maps
// to another contract is never asked for an older one's history. The
// vendor's continuous near-month series is filed as its own series,
// `<UNDERLYING>-FUT-C1`, exactly as answered: the vendor states no roll day,
// so it is never split into contracts. Nothing here is swept:
// `Instrument::is_sweepable` refuses every contract and every FNO key.

use crate::master_snapshots::{Book, Listed, Stamp, TokenOn, first_listed};
use crate::session::{DropCensus, DropReason, Window};
use crate::vendor::{ContinuousSpec, HttpSpec};

/// The continuous series' symbol suffix: `<UNDERLYING>-FUT-C1`.
pub const CONTINUOUS_SUFFIX: &str = "-FUT-C1";

/// The `/logs` target of every futures plan refusal, request and landing.
pub const FUTURES_EVENT: &str = "pull.futures";

/// The symbol the continuous series is filed under.
///
/// # Errors
///
/// A name the store cannot file under (too long, or a byte a symbol may not
/// carry).
pub fn continuous_symbol(underlying: &str) -> Result<Symbol, InstrumentError> {
    Symbol::new(&format!("{underlying}{CONTINUOUS_SUFFIX}"))
}

/// What one futures request is filed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuturesSeries {
    /// One contract, keyed (underlying, expiry).
    Contract {
        /// The contract segment, `<expiry>-FUT`.
        contract: brutex_core::instrument::Contract,
        /// The expiry the dated snapshot states.
        expiry: Day,
    },
    /// The vendor's continuous near-month series, as answered.
    Continuous,
}

impl FuturesSeries {
    /// How a receipt and `/logs` name it.
    #[must_use]
    pub fn label(&self, underlying: &str) -> String {
        match *self {
            Self::Contract { contract, .. } => format!("{underlying} {contract}"),
            Self::Continuous => format!("{underlying}{CONTINUOUS_SUFFIX}"),
        }
    }
}

/// One futures request: the address, and what the answer is filed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuturesAsk {
    /// The vendor's token, resolved through a dated snapshot.
    pub token: u64,
    /// What the answer is filed under.
    pub series: FuturesSeries,
    /// The days asked for, inclusive.
    pub window: Window,
}

/// What a futures pull will ask, and what it would not ask, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuturesPlan {
    /// The underlying.
    pub underlying: String,
    /// The newest snapshot every token was checked against.
    pub newest: Stamp,
    /// The requests, contracts by expiry first, the continuous series last.
    pub asks: Vec<FuturesAsk>,
    /// Every contract or series not asked for, and why.
    pub refused: Vec<String>,
    /// Why the futures first-listed index was rebuilt as a new generation
    /// before this plan was read from it, when it did not match its commit log
    /// (R4-28 repair 2).
    pub rebuilt: Option<String>,
    /// Every Kite snapshot entry the book's listing named incomplete (a write
    /// cut before its receipt), newest first: never read by this plan, so a
    /// day it would have dated resolves from its neighbours (R4-28 repair 3).
    /// At most [`INCOMPLETE_NAMED`](crate::master_snapshots::INCOMPLETE_NAMED)
    /// are named, and one line more counts the rest.
    pub incomplete: Vec<String>,
}

/// The book's incomplete Kite entries as a plan names them: newest first, at
/// most [`INCOMPLETE_NAMED`](crate::master_snapshots::INCOMPLETE_NAMED), and
/// one line counting the rest.
fn incomplete_lines(book: &Book) -> Vec<String> {
    let held = book.incomplete();
    let bound = crate::master_snapshots::INCOMPLETE_NAMED;
    let mut lines: Vec<String> = held
        .iter()
        .rev()
        .take(bound)
        .map(ToString::to_string)
        .collect();
    if held.len() > bound {
        lines.push(format!(
            "{} older incomplete entries are held and not named here",
            held.len().saturating_sub(bound)
        ));
    }
    lines
}

/// Whether `listed` is a future of `underlying` on the route's exchange.
fn is_future_of(listed: &Listed<'_>, route: ContinuousSpec, underlying: &str) -> bool {
    listed.exchange() == route.exchange
        && listed.instrument_type() == route.future_type
        && listed.name() == underlying
        && listed.expiry().is_some()
}

/// The contract segment of a future expiring on `expiry`.
fn future_contract(expiry: Day) -> Option<brutex_core::instrument::Contract> {
    let expiry =
        brutex_core::instrument::Expiry::new(expiry.year(), expiry.month(), expiry.day()).ok()?;
    brutex_core::instrument::Contract::of(brutex_core::instrument::Kind::Future { expiry })
}

/// Plans one underlying's futures pull over `window`.
///
/// Each contract is asked from the first snapshot that lists it (a token is
/// believed to name a contract only from a snapshot that says so) to its
/// expiry, inside the window. That first day is read from the
/// [`first_listed`](crate::master_snapshots::first_listed) index, which every
/// snapshot write fills, so no older snapshot is opened here: the cost does not
/// grow with the snapshots held (R4-28 repair 1; D-7187). A snapshot on or
/// before the window's end that the index does not hold yet is taken in first,
/// once; one that cannot be is refused by name. An index that does not match
/// its commit log (a file missing, cut or garbled) is never planned from: a
/// new generation is built from the receipted snapshots first and named in
/// [`FuturesPlan::rebuilt`], and one that still does not match is refused
/// `futures_index_damaged` (R4-28 repair 2). Each token is then checked
/// against the NEWEST snapshot, the one snapshot a plan parses: one that
/// now names another contract is refused `token_reused` and never asked for
/// the older contract's history; one the newest snapshot no longer lists
/// (flushed after expiry) is asked, and an answer of nothing ends that
/// contract's history by name. The continuous series is asked through the
/// newest snapshot's nearest future expiring on or after `today`.
///
/// # Errors
///
/// No receipted snapshot at all (`no_snapshot`): nothing is planned from an
/// undated master. A newest snapshot dated after `today`
/// (`snapshot_from_the_future`). A book built in memory, or an index that
/// cannot be locked, read or appended to (`futures_index`): nothing is
/// planned from a partial answer. Each is one Error [`FUTURES_EVENT`] event
/// too, naming the underlying and the refusal (R4-28 repair 5, the r5
/// check's follow-up: they reached the receipt and `/audit` only).
pub fn futures_plan(
    book: &Book,
    route: ContinuousSpec,
    underlying: &str,
    window: Window,
    today: Day,
    last_settled: Day,
) -> Result<FuturesPlan, String> {
    plan_whole(book, route, underlying, window, today, last_settled).inspect_err(|why| {
        let _ = telemetry::emit_if!(
            telemetry::Level::Error,
            FUTURES_EVENT,
            "no future of the underlying was planned",
            "underlying" => telemetry::Value::Str(underlying),
            "why" => telemetry::Value::Str(why),
        );
    })
}

/// [`futures_plan`], before its refusal is emitted.
fn plan_whole(
    book: &Book,
    route: ContinuousSpec,
    underlying: &str,
    window: Window,
    today: Day,
    last_settled: Day,
) -> Result<FuturesPlan, String> {
    let Some(newest) = book.newest() else {
        return Err(format!(
            "no_snapshot: no dated Kite master snapshot is held, so no futures token of \
             {underlying} can be resolved by date; refresh the masters (POST /masters/refresh) \
             first"
        ));
    };
    // A NEWEST SNAPSHOT FROM THE FUTURE IS NEVER PLANNED FROM (R4-28 repair
    // 4, the r4 check's follow-up), for the reason `newest_snapshot_day`
    // refuses it: the clock that dated it ran ahead, and every token here is
    // checked against the newest dump and the near month chosen from it.
    if newest.day > today {
        return Err(format!(
            "snapshot_from_the_future: the newest dated Kite master snapshot {newest} is dated \
             after today {today}: the clock that wrote it ran ahead or this one runs behind, so \
             no future of {underlying} was planned from it. Move the future-dated snapshot and \
             its receipt aside by hand to resume"
        ));
    }
    let through = window.to().min(last_settled);
    let listing = book
        .root()
        .ok_or_else(|| "a book built in memory has no snapshots directory".to_owned())
        .and_then(|root| first_listed::listing(root, book, route, underlying, through))
        .map_err(|why| format!("futures_index: {why}; no future of {underlying} was planned"))?;
    if let Some(ref why) = listing.rebuilt {
        let _ = telemetry::emit_if!(
            telemetry::Level::Error,
            FUTURES_EVENT,
            "the futures first-listed index did not match its commit log, so this plan was read \
             from a new generation built again from the receipted snapshots; the old one is left \
             as it was",
            "underlying" => telemetry::Value::Str(underlying),
            "why" => telemetry::Value::Str(why),
        );
    }
    let mut refused = listing.refused;
    let first_seen = listing.first;
    // ONE CONTRACT, ONE TOKEN (R4-28 repair 3, the r3 check's follow-up): a
    // dump that lists one (underlying, expiry) under two tokens would file two
    // series as one contract's, so neither is asked and both are named.
    let mut tokens_of: std::collections::BTreeMap<Day, Vec<u64>> =
        std::collections::BTreeMap::new();
    for &(expiry, token) in first_seen.keys() {
        tokens_of.entry(expiry).or_default().push(token);
    }
    let mut asks = Vec::with_capacity(first_seen.len().saturating_add(1));
    for (&(expiry, token), &seen) in &first_seen {
        let (from, to) = (seen.max(window.from()), expiry.min(through));
        let Ok(span) = Window::new(from, to) else {
            continue;
        };
        let tokens = tokens_of.get(&expiry).map_or(&[][..], Vec::as_slice);
        match contract_ask(
            book,
            route,
            underlying,
            newest,
            tokens,
            (expiry, token, seen),
            span,
        ) {
            Ok(ask) => asks.push(ask),
            Err(why) => refused.push(why),
        }
    }
    match continuous_ask(book, route, underlying, today, window.from(), through) {
        Ok(Some(ask)) => asks.push(ask),
        Ok(None) => {}
        Err(why) => refused.push(why),
    }
    for why in &refused {
        let _ = telemetry::emit_if!(
            telemetry::Level::Warn,
            FUTURES_EVENT,
            "a futures contract or series was not asked for",
            "underlying" => telemetry::Value::Str(underlying),
            "why" => telemetry::Value::Str(why),
        );
    }
    Ok(FuturesPlan {
        underlying: underlying.to_owned(),
        newest,
        asks,
        refused,
        rebuilt: listing.rebuilt,
        incomplete: incomplete_lines(book),
    })
}

/// One contract's request over `span`, or why it is not asked, by name.
///
/// `tokens` is every token the index lists for this contract's expiry: more
/// than one is `contract_duplicated` (R4-28 repair 3, the r3 check's
/// follow-up), since a dump that lists one (underlying, expiry) under two
/// tokens would file two series as one contract's, so neither is asked and
/// both are named. The token is then checked against the newest snapshot.
fn contract_ask(
    book: &Book,
    route: ContinuousSpec,
    underlying: &str,
    newest: Stamp,
    tokens: &[u64],
    (expiry, token, seen): (Day, u64, Day),
    span: Window,
) -> Result<FuturesAsk, String> {
    let label = format!("{underlying} {expiry} future (token {token}, first listed {seen})");
    if tokens.len() > 1 {
        let tokens: Vec<String> = tokens.iter().map(u64::to_string).collect();
        return Err(format!(
            "contract_duplicated: {label}: the index lists this one contract under {} tokens \
             ({}), so none is asked and no two series are filed as one",
            tokens.len(),
            tokens.join(", ")
        ));
    }
    // IDENTITY, NOT TRADING: the newest snapshot may still list a contract
    // past its expiry (`token_expired` on that day, R4-28 repair 3), and the
    // same contract is still what the token names in its history.
    match book.token_on(newest.day, route.exchange, token) {
        TokenOn::Resolved { listed, .. } | TokenOn::Expired { listed, .. }
            if is_future_of(&listed, route, underlying) && listed.expiry() == Some(expiry) => {}
        TokenOn::NotListed { .. } => {}
        TokenOn::Resolved { listed, .. } | TokenOn::Expired { listed, .. } => {
            return Err(format!(
                "token_reused: {label}: the newest snapshot {newest} maps the token to {}, so its \
                 history is not asked through it",
                listed.tradingsymbol()
            ));
        }
        other => {
            return Err(format!(
                "{}: {label}, checked against the newest snapshot {newest}",
                other.word()
            ));
        }
    }
    let contract = future_contract(expiry)
        .ok_or_else(|| format!("contract_unnamed: {label}: no contract segment renders"))?;
    Ok(FuturesAsk {
        token,
        series: FuturesSeries::Contract { contract, expiry },
        window: span,
    })
}

/// The continuous series' request: through the newest snapshot's nearest
/// future of `underlying` expiring on or after `today`, over `from..=through`.
/// `Ok(None)` when that span is empty; a refusal, by name, when no live
/// future is listed or the newest snapshot does not read.
fn continuous_ask(
    book: &Book,
    route: ContinuousSpec,
    underlying: &str,
    today: Day,
    from: Day,
    through: Day,
) -> Result<Option<FuturesAsk>, String> {
    let Some(newest) = book.newest() else {
        return Err("no_snapshot: no dated Kite master snapshot is held".to_owned());
    };
    let last = book.stamps().len().saturating_sub(1);
    let near = book
        .with_snapshot(last, |snapshot| {
            snapshot
                .rows()
                .filter(|listed| is_future_of(listed, route, underlying))
                .filter(|listed| listed.expiry().is_some_and(|expiry| expiry >= today))
                .min_by_key(|listed| (listed.expiry(), listed.token()))
                .map(|listed| listed.token())
        })
        .map_err(|why| format!("snapshot_unreadable: {newest}: {why}"))?;
    let Some(token) = near else {
        return Err(format!(
            "no_live_future: the newest snapshot {newest} lists no {} future of {underlying} \
             expiring on or after {today}, so the continuous series has no token to ask through",
            route.exchange
        ));
    };
    Ok(Window::new(from, through).ok().map(|span| FuturesAsk {
        token,
        series: FuturesSeries::Continuous,
        window: span,
    }))
}

/// What one futures pull did: what it read, what it offered the store, what
/// the store WROTE, and where every other row went.
///
/// # Offered is not written (R4-28 repair 4; D-7190)
///
/// [`crate::ingest::Ingested`] keeps two counts because they part on every
/// re-pull: `bars_stored` is what a landing OFFERED the store and
/// `bars_committed` what `Appended::Committed` wrote. This type used to carry
/// the first under the words "bars written", so an identical re-pull that
/// wrote nothing answered "4 bars stored" and was journalled `Stored`, the
/// misreport `bars_committed` exists to end, and a row the landing dropped (a
/// day the calendar records closed, a day before or after the window asked)
/// was counted nowhere on the receipt. Both counts are carried under their
/// own names now, with the fold count and the drop census beside them, so
/// [`Self::balances`] can account for every row read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FuturesLanded {
    /// Requests sent.
    pub requests: usize,
    /// Rows the vendor answered with, decoded.
    pub rows_read: usize,
    /// Bars OFFERED to the store (`Ingested::bars_stored`), not necessarily
    /// written: a re-pull offers every bar again.
    pub bars_offered: usize,
    /// Bars this pull WROTE (`Ingested::bars_committed`): zero on a re-pull
    /// of days the store already holds byte for byte.
    pub bars_committed: usize,
    /// Rows folded into a day bar already open: one day answered twice.
    pub rows_folded: usize,
    /// Every row that did not become a bar, by reason.
    pub census: DropCensus,
    /// One line per series asked: what it is and what came of it.
    pub series: Vec<String>,
    /// Every series that did not land whole, and why.
    pub failed: Vec<String>,
}

impl FuturesLanded {
    /// Whether every row read is accounted for (offered to the store, folded
    /// into an open bar, or dropped by reason) and no series stopped.
    ///
    /// The same check as `Ingested::balances`, over the whole pull. A row
    /// that is in none of the three is indistinguishable from one the vendor
    /// never sent.
    #[must_use]
    pub fn balances(&self) -> bool {
        self.failed.is_empty()
            && self.rows_read
                == self
                    .bars_offered
                    .saturating_add(self.rows_folded)
                    .saturating_add(self.census.total() as usize)
    }

    /// The receipt's balance line, in the spot receipt's words.
    #[must_use]
    pub fn balance_line(&self) -> String {
        if self.balances() {
            format!(
                "yes — {} read = {} offered + {} folded + {} dropped",
                self.rows_read,
                self.bars_offered,
                self.rows_folded,
                self.census.total()
            )
        } else {
            format!(
                "NO — {} rows read, {} offered, {} folded, {} dropped, {} series stopped",
                self.rows_read,
                self.bars_offered,
                self.rows_folded,
                self.census.total(),
                self.failed.len()
            )
        }
    }

    /// Every reason that dropped a row, with its count; `none` when nothing
    /// was dropped.
    #[must_use]
    pub fn dropped_by_reason(&self) -> String {
        dropped_by_reason(&self.census)
    }

    /// Adds one series' outcome to the pull's totals.
    fn absorb(&mut self, one: &SeriesOutcome) {
        self.requests = self.requests.saturating_add(one.requests);
        self.rows_read = self.rows_read.saturating_add(one.read);
        self.bars_offered = self.bars_offered.saturating_add(one.offered);
        self.bars_committed = self.bars_committed.saturating_add(one.committed);
        self.rows_folded = self.rows_folded.saturating_add(one.folded);
        self.census.absorb(one.census);
    }
}

/// The reasons a futures receipt names, in the order `/audit` shows them.
const DROP_REASONS: [DropReason; 5] = [
    DropReason::BeforeWindow,
    DropReason::AfterWindow,
    DropReason::BeforeSessionOpen,
    DropReason::AtOrAfterSessionClose,
    DropReason::OnClosedDay,
];

/// Each reason in `census` that dropped a row, with its count, joined by
/// ` · `; `none` when nothing was dropped.
///
/// The names explain the total and never replace it: every surface prints
/// `census.total()` beside this line, so a drop under a reason this list did
/// not yet name would still be counted there.
fn dropped_by_reason(census: &DropCensus) -> String {
    let named: Vec<String> = DROP_REASONS
        .iter()
        .filter_map(|reason| {
            let n = census.of(*reason);
            (n > 0).then(|| format!("{} {n}", reason.label()))
        })
        .collect();
    if named.is_empty() {
        "none".to_owned()
    } else {
        named.join(" · ")
    }
}

/// Asks the vendor for every request of `plan` and lands each answer under
/// its own series, open interest kept.
///
/// Each request is cut to the route's window cap. A refusal, an answer that
/// will not decode, or an answer of nothing for a contract ends that series by
/// name (an expired token the vendor no longer serves is exactly this), and the
/// pull goes on to the next series. One [`FUTURES_EVENT`] event per series.
///
/// # Cost
///
/// One request per window chunk; one landing per answer, through the same
/// `ingest::from_window` every pull uses.
pub async fn pull_futures_day<D: crate::chain::Discovery>(
    from: &D,
    spec: &HttpSpec,
    plan: &FuturesPlan,
    store_root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    origin: &str,
) -> FuturesLanded {
    let mut out = FuturesLanded::default();
    let Some(route) = spec.fno.by_token() else {
        out.failed
            .push("this feed has no futures token route; nothing was asked".to_owned());
        return out;
    };
    let landing = Landing {
        store_root,
        vendor,
        origin,
    };
    for ask in &plan.asks {
        let label = ask.series.label(&plan.underlying);
        let window = ask.window_text();
        let one = pull_one(from, spec, route, &plan.underlying, ask, landing).await;
        out.absorb(&one);
        let line = one.line(&label, &window);
        let (level, message) = if one.stopped.is_some() {
            (telemetry::Level::Error, "a futures series stopped by name")
        } else {
            (telemetry::Level::Info, "a futures series landed")
        };
        let token = ask.token.to_string();
        let dropped_by = dropped_by_reason(&one.census);
        // THE EVENT COUNTS WHAT WAS WRITTEN, BESIDE WHAT WAS OFFERED, and
        // carries the receipt's own line, so `/logs` and the receipt cannot
        // disagree about one series (R4-28 repair 4; D-7190).
        let _ = telemetry::emit_if!(
            level,
            FUTURES_EVENT,
            message,
            "series" => telemetry::Value::Str(&label),
            "token" => telemetry::Value::Str(&token),
            "window" => telemetry::Value::Str(&window),
            "rows" => telemetry::Value::Uint(one.read as u64),
            "offered" => telemetry::Value::Uint(one.offered as u64),
            "written" => telemetry::Value::Uint(one.committed as u64),
            "folded" => telemetry::Value::Uint(one.folded as u64),
            "dropped" => telemetry::Value::Uint(u64::from(one.census.total())),
            "dropped_by" => telemetry::Value::Str(&dropped_by),
            "line" => telemetry::Value::Str(&line),
            "detail" => telemetry::Value::Str(one.stopped.as_deref().unwrap_or("")),
        );
        if let Some(why) = one.stopped {
            out.failed.push(format!("{label}: {why}"));
        }
        out.series.push(line);
    }
    out
}

impl FuturesAsk {
    /// `from..to`, as a receipt prints it.
    #[must_use]
    pub fn window_text(&self) -> String {
        format!("{}..{}", self.window.from(), self.window.to())
    }
}

/// Where answers land.
#[derive(Clone, Copy)]
struct Landing<'a> {
    store_root: &'a std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    origin: &'a str,
}

/// What one series came to, summed over its chunks from each landing's own
/// `Ingested`.
#[derive(Default)]
struct SeriesOutcome {
    /// Bars offered to the store (`Ingested::bars_stored`).
    offered: usize,
    /// Bars the store wrote (`Ingested::bars_committed`).
    committed: usize,
    /// Rows folded into a bar already open (`Ingested::rows_folded`).
    folded: usize,
    /// Rows dropped, by reason (`Ingested::census`).
    census: DropCensus,
    read: usize,
    requests: usize,
    stopped: Option<String>,
}

impl SeriesOutcome {
    fn stop(mut self, why: String) -> Self {
        self.stopped = Some(why);
        self
    }

    /// Adds one chunk's landing.
    fn absorb(&mut self, landed: &crate::ingest::Ingested) {
        self.offered = self.offered.saturating_add(landed.bars_stored);
        self.committed = self.committed.saturating_add(landed.bars_committed);
        self.folded = self.folded.saturating_add(landed.rows_folded);
        self.census.absorb(landed.census);
    }

    /// The receipt's line for this series: what was WRITTEN first, what was
    /// offered beside it, and where every other row went.
    ///
    /// "already held" is the offered bars the store did not write: on a
    /// landing that did not fail, `BarFile::append` wrote only what it did not
    /// already hold byte for byte, so that is exactly what the difference is.
    /// On a series that stopped it is not claimed.
    fn line(&self, label: &str, window: &str) -> String {
        let held = self.offered.saturating_sub(self.committed);
        let held = if held > 0 && self.stopped.is_none() {
            format!(" ({held} already held)")
        } else {
            String::new()
        };
        let dropped = self.census.total();
        let dropped = if dropped == 0 {
            "0 dropped".to_owned()
        } else {
            format!("{dropped} dropped ({})", dropped_by_reason(&self.census))
        };
        let stopped = self
            .stopped
            .as_ref()
            .map_or_else(String::new, |why| format!("; then stopped: {why}"));
        format!(
            "{label}: {} bars written, {} offered{held} from {} rows over {window}; \
             {} folded into an open bar; {dropped}{stopped}",
            self.committed, self.offered, self.read, self.folded
        )
    }
}

/// Runs blocking store work off the async workers when there are workers to
/// keep free: `block_in_place` on a multi-thread runtime, which hands this
/// worker's other tasks to another while `work` runs; inline on a
/// current-thread runtime (`block_in_place` panics there, and there is no
/// other worker to starve) and outside any runtime.
fn off_the_workers<T>(work: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current().map(|handle| handle.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}

/// Asks and lands one series, chunk by chunk.
async fn pull_one<D: crate::chain::Discovery>(
    from: &D,
    spec: &HttpSpec,
    route: ContinuousSpec,
    underlying: &str,
    ask: &FuturesAsk,
    landing: Landing<'_>,
) -> SeriesOutcome {
    let mut out = SeriesOutcome::default();
    let (instrument, contract) = match ask.series {
        FuturesSeries::Contract { contract, .. } => (underlying.to_owned(), Some(contract)),
        FuturesSeries::Continuous => match continuous_symbol(underlying) {
            Ok(symbol) => (symbol.as_str().to_owned(), None),
            Err(why) => return out.stop(format!("the continuous symbol does not file: {why}")),
        },
    };
    let chunks =
        match crate::session::split_window(ask.window, spec.window_cap_days(route.granularity)) {
            Ok(chunks) => chunks,
            Err(why) => return out.stop(format!("the window does not split to the cap: {why}")),
        };
    let continuous = matches!(ask.series, FuturesSeries::Continuous);
    for chunk in chunks {
        let span = format!("{}..{}", chunk.from(), chunk.to());
        let url = match crate::fno::day_candles_url(spec, ask.token, chunk, continuous) {
            Ok(url) => url,
            Err(why) => return out.stop(why),
        };
        out.requests = out.requests.saturating_add(1);
        let body = match from.get(&url).await {
            Ok(body) => body,
            Err(refusal) => {
                return out.stop(format!(
                    "vendor_refused: the vendor refused {span}: {refusal}; nothing after it was asked for this series"
                ));
            }
        };
        let raw = match crate::http::decode_body(&body, spec, crate::vendor::Listing::Derivative) {
            Ok(raw) => raw,
            Err(why) => return out.stop(format!("the answer for {span} does not decode: {why}")),
        };
        out.read = out.read.saturating_add(raw.rows.len());
        if raw.rows.is_empty() {
            if contract.is_some() {
                return out.stop(format!(
                    "history_ends: the vendor answered no candle for {span}"
                ));
            }
            continue;
        }
        let request = crate::fetch::BarRequest {
            instrument_id: ask.token.to_string(),
            listing: crate::vendor::Listing::Derivative,
            window: chunk,
            granularity: route.granularity,
        };
        // THE LANDING IS BLOCKING FILE WORK (bar files, the census, fsync),
        // so it runs off the async workers when there are workers to keep
        // free (R4-28 repair 4, the r4 check's follow-up).
        let landed = off_the_workers(|| {
            crate::ingest::from_window(
                &raw,
                &instrument,
                landing.origin,
                landing.store_root,
                crate::ingest::Plan {
                    calendar: crate::calendar::Runtime::default(),
                    cash_schedule: None,
                    columns: crate::csv::Columns::Gdfl,
                    request: &request,
                    encoding: spec.timestamps,
                    scale: crate::http::DECODED_PRICE_SCALE,
                    vendor: landing.vendor,
                    exchange: VENUE.as_str(),
                    segment: Segment::Fno.as_str(),
                    contract,
                    // No runs-ledger id reaches a futures day pull yet (R4-28
                    // and IA-10c landed apart): `0` is `Plan.run`'s documented
                    // "no ledger id", so its lines name the sink's claimed run
                    // as before. Carrying the request's id here is a follow-up.
                    run: 0,
                },
            )
        });
        out.absorb(&landed);
        if let Some(failure) = landed.failures.first() {
            let why = failure.why.clone();
            return out.stop(format!("{span} did not land whole: {why}"));
        }
    }
    out
}

#[cfg(test)]
#[path = "fnowork_futures_tests.rs"]
mod futures_tests;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
    use brutex_core::price::Paisa;

    fn month(year: u16, month: u8) -> YearMonth {
        YearMonth::new(year, month).expect("a real month")
    }

    fn day(year: u16, m: u8, d: u8) -> Day {
        Day::new(year, m, d).expect("a real day")
    }

    /// 15:29 IST on `on`, as the epoch micros the manifest stores.
    ///
    /// IST is UTC+5:30, so the UTC second is that day's midnight less 19,800
    /// plus the minute of day. Round-tripped by the unit below rather than
    /// trusted — an arithmetic slip here would make every assertion beneath it
    /// agree with the same wrong day.
    fn stamp(on: Day) -> i64 {
        let secs = i64::from(on.days_from_epoch()) * 86_400 - 19_800 + (15 * 3_600 + 29 * 60);
        secs * 1_000_000
    }

    #[test]
    fn the_test_stamp_round_trips_through_the_reader_it_feeds() {
        for d in [day(2025, 11, 1), day(2026, 1, 15), day(2026, 1, 31)] {
            assert_eq!(day_of(stamp(d)), Some(d), "{d} did not survive the trip");
        }
    }

    /// **THE REGRESSION THAT WITHDREW THE BOOLEAN GATE.**
    ///
    /// A month pulled to day five, then interrupted. `gaps_by` — which asks
    /// only whether the key exists — calls it held and fetches nothing, so the
    /// remaining twenty-six days are reported complete and, under §8's
    /// append-only rule, can never be filled by re-running.
    ///
    /// `owed` sees the same manifest and answers with a POSITION.
    #[test]
    fn a_month_pulled_to_day_five_is_short_where_the_boolean_gate_called_it_held() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );
        let window = (day(2026, 1, 1), day(2026, 1, 31));

        // WHAT THE WITHDRAWN GATE SAW: a key that exists, therefore done.
        let flagged = gaps_by(&cells.cells, |_| true);
        assert!(
            flagged.is_complete(),
            "the boolean gate cannot see a partial month — this assertion is \
             the bug, kept so the contrast below is a fact rather than a claim"
        );

        // WHAT A POSITION SEES.
        let out = owed(&cells.cells, window, |_| Some(stamp(day(2026, 1, 5))));
        assert_eq!(out.complete, 0);
        assert_eq!(out.resume.len(), 1);
        assert_eq!(
            out.resume[0].from,
            day(2026, 1, 6),
            "the day AFTER the last held"
        );
        assert_eq!(
            out.resume[0].through,
            day(2026, 1, 29),
            "the expiry, not the 31st"
        );
    }

    /// **THE EXPIRY CLAMP.** Held to the expiry is held to the end.
    ///
    /// Without it the contract's final month reads short on every run: it stops
    /// trading on the 29th and no vendor will ever send the 30th or 31st, so
    /// the same fetch is reissued forever and returns empty each time.
    #[test]
    fn a_contract_held_through_its_expiry_owes_nothing_more_of_that_month() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );
        let window = (day(2026, 1, 1), day(2026, 1, 31));

        let out = owed(&cells.cells, window, |_| Some(stamp(day(2026, 1, 29))));
        assert!(
            out.is_complete(),
            "the 30th and 31st are not this contract's to owe"
        );
        assert_eq!(out.complete, 1);

        // ONE DAY SHORT OF THE EXPIRY IS STILL SHORT.
        let out = owed(&cells.cells, window, |_| Some(stamp(day(2026, 1, 28))));
        assert_eq!(out.resume.len(), 1);
        assert_eq!(out.resume[0].from, day(2026, 1, 29));
    }

    /// A key the manifest does not carry owes its whole span.
    ///
    /// `None` is not "complete and empty" — nothing has ever been written for
    /// it, which is the strongest reason to ask.
    #[test]
    fn a_key_the_manifest_does_not_carry_owes_the_whole_span() {
        let dec = month(2025, 12);
        let cells = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[dec],
            Timeframe::MINUTE_1,
        );
        let out = owed(&cells.cells, (day(2025, 11, 20), day(2026, 1, 31)), |_| {
            None
        });

        assert_eq!(out.resume.len(), 1);
        assert_eq!(
            out.resume[0].from,
            day(2025, 12, 1),
            "the month opens after the window does"
        );
        assert_eq!(
            out.resume[0].through,
            day(2025, 12, 31),
            "a January expiry does not clamp a December month — the month end does"
        );
    }

    /// The window clamps on both sides, and the month clamps inside it.
    #[test]
    fn the_span_is_the_intersection_of_the_month_the_window_and_the_expiry() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );

        // A window that opens mid-month and closes before the expiry.
        let out = owed(&cells.cells, (day(2026, 1, 12), day(2026, 1, 20)), |_| None);
        assert_eq!(
            out.resume[0].from,
            day(2026, 1, 12),
            "the window opens after the month"
        );
        assert_eq!(
            out.resume[0].through,
            day(2026, 1, 20),
            "the window closes before the expiry"
        );

        // Held past the window's end is held enough, even though the contract
        // itself traded for nine days more.
        let out = owed(&cells.cells, (day(2026, 1, 12), day(2026, 1, 20)), |_| {
            Some(stamp(day(2026, 1, 20)))
        });
        assert!(out.is_complete(), "the operator did not ask for the 21st");
    }

    /// A resume point BEFORE the window is clamped up to it, never asked early.
    #[test]
    fn a_resume_point_before_the_window_opens_is_clamped_up_to_it() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );

        // Held through the 3rd, but the operator asked from the 12th. Asking
        // from the 4th would write eight days they did not request.
        let out = owed(&cells.cells, (day(2026, 1, 12), day(2026, 1, 20)), |_| {
            Some(stamp(day(2026, 1, 3)))
        });
        assert_eq!(out.resume.len(), 1);
        assert_eq!(out.resume[0].from, day(2026, 1, 12));
    }

    /// A cell the window and the expiry leave no day in owes nothing.
    #[test]
    fn a_cell_with_no_fetchable_day_is_complete_rather_than_asked_for() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );

        // The window opens after the contract expired.
        let out = owed(&cells.cells, (day(2026, 1, 30), day(2026, 2, 28)), |_| None);
        assert!(
            out.is_complete(),
            "nothing traded after the 29th to ask for"
        );
        assert_eq!(out.complete, 1);
        assert_eq!(out.outside_window, 1);

        // The window closes before the month opens.
        let out = owed(&cells.cells, (day(2025, 1, 1), day(2025, 6, 30)), |_| None);
        assert!(out.is_complete());
        assert_eq!(out.outside_window, 1);
    }

    #[test]
    fn complete_cells_distinguish_held_history_from_days_never_owed() {
        let discovered = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[month(2025, 12), month(2026, 1), month(2026, 2)],
            Timeframe::MINUTE_1,
        );
        let probes = std::cell::Cell::new(0);
        for held in [None, Some(stamp(day(2026, 1, 29)))] {
            probes.set(0);
            let out = owed(
                &discovered.cells,
                (day(2026, 1, 1), day(2026, 2, 28)),
                |key| {
                    assert_eq!(key.month, month(2026, 1));
                    probes.set(probes.get() + 1);
                    held
                },
            );
            assert_eq!(probes.get(), 1, "only a fetchable month probes history");
            assert_eq!(out.offered(), 3);
            assert_eq!(out.outside_window, 2);
            assert_eq!(out.complete, if held.is_some() { 3 } else { 2 });
            assert_eq!(out.resume.len(), usize::from(held.is_none()));
            assert_eq!(out.is_complete(), held.is_some());
        }
    }

    /// Every offered cell is accounted for, in either column.
    ///
    /// A count rather than a spot check: the failure this guards is a cell
    /// falling out of both — neither fetched nor reported — which no example
    /// test notices.
    #[test]
    fn every_offered_cell_lands_in_exactly_one_column() {
        let months = [month(2025, 11), month(2025, 12), month(2026, 1)];
        let cells = cells(
            &[
                found("BANKNIFTY", 29, 5_800_000),
                found("NIFTY", 29, 2_600_000),
            ],
            &months,
            Timeframe::MINUTE_1,
        );
        assert_eq!(cells.cells.len(), 6);

        // A mixed manifest: nothing for the odd cells, complete for the even.
        let seen = std::cell::Cell::new(0usize);
        let out = owed(&cells.cells, (day(2025, 11, 1), day(2026, 1, 31)), |_| {
            let n = seen.get();
            seen.set(n + 1);
            (n % 2 == 1).then(|| stamp(day(2026, 1, 31)))
        });
        assert_eq!(out.offered(), 6, "three cells each for two contracts");
        assert_eq!(out.resume.len() + out.complete, 6);
    }

    /// An empty round is complete, not one empty row.
    #[test]
    fn nothing_offered_is_complete_and_offers_nothing() {
        let out = owed(&[], (day(2026, 1, 1), day(2026, 1, 31)), |_| None);
        assert!(out.is_complete());
        assert_eq!(out.offered(), 0);
        assert_eq!(out, Owed::default());
    }

    /// A stamp no calendar can read is treated as nothing held.
    ///
    /// The alternative is worse in both directions: trusting it would resume
    /// from a day that does not exist, and refusing the cell would drop it from
    /// both columns.
    #[test]
    fn a_stamp_outside_the_calendar_reads_as_nothing_held() {
        let jan = month(2026, 1);
        let cells = cells(
            &[found("NIFTY", 29, 2_600_000)],
            &[jan],
            Timeframe::MINUTE_1,
        );
        assert_eq!(day_of(i64::MIN), None, "the reader must refuse it first");

        let out = owed(&cells.cells, (day(2026, 1, 1), day(2026, 1, 31)), |_| {
            Some(i64::MIN)
        });
        assert_eq!(out.resume.len(), 1);
        assert_eq!(
            out.resume[0].from,
            day(2026, 1, 1),
            "the whole span, as for a key never seen"
        );
    }

    fn found(underlying: &str, day: u8, strike: i64) -> Found {
        let expiry = Expiry::new(2026, 1, day).expect("a real expiry");
        let kind = Kind::Option {
            expiry,
            strike: Paisa::from_raw(strike),
            side: OptionSide::Call,
        };
        Found {
            vendor_symbol: format!("{underlying}26JAN{strike}CE"),
            underlying: underlying.to_owned(),
            contract: Contract::of(kind).expect("an option has a contract segment"),
            expiry,
            option: Some((Paisa::from_raw(strike), OptionSide::Call)),
        }
    }

    #[test]
    fn the_ladder_is_underlying_major_and_month_ascending() {
        let jan = month(2026, 1);
        let feb = month(2026, 2);
        let work = ladder(&["BANKNIFTY", "NIFTY"], &[jan, feb]);

        // The ORDER is the invariant, not merely the contents: a later month
        // fetched first permanently blocks the earlier days of that month file,
        // which is what `store::file::BarFile::append` refuses.
        assert_eq!(
            work,
            vec![
                Ask {
                    underlying: "BANKNIFTY".to_owned(),
                    month: jan
                },
                Ask {
                    underlying: "BANKNIFTY".to_owned(),
                    month: feb
                },
                Ask {
                    underlying: "NIFTY".to_owned(),
                    month: jan
                },
                Ask {
                    underlying: "NIFTY".to_owned(),
                    month: feb
                },
            ]
        );
    }

    #[test]
    fn a_ladder_over_no_underlyings_or_no_months_is_empty_rather_than_one_row() {
        let jan = month(2026, 1);
        assert!(ladder(&[], &[jan]).is_empty());
        assert!(ladder(&["BANKNIFTY"], &[]).is_empty());
        assert!(ladder(&[], &[]).is_empty());
    }

    #[test]
    fn one_contract_over_three_months_is_three_cells_that_differ_only_in_the_month() {
        let months = [month(2025, 11), month(2025, 12), month(2026, 1)];
        let out = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &months,
            Timeframe::MINUTE_1,
        );

        assert_eq!(out.cells.len(), 3);
        assert!(out.refused.is_empty());
        for (cell, expected) in out.cells.iter().zip(months) {
            assert_eq!(cell.key.month, expected);
            assert_eq!(cell.key.segment, Segment::Fno);
            assert_eq!(cell.key.exchange, Exchange::Nse);
            assert_eq!(cell.vendor_symbol, "BANKNIFTY26JAN5800000CE");
        }
        // The contract segment is what separates two strikes of one month, so
        // every cell of ONE contract must carry the same one.
        assert_eq!(out.cells[0].key.contract, out.cells[2].key.contract);
    }

    #[test]
    fn two_strikes_of_one_underlying_do_not_collide_on_one_key() {
        let jan = [month(2026, 1)];
        let out = cells(
            &[
                found("BANKNIFTY", 29, 5_800_000),
                found("BANKNIFTY", 29, 5_810_000),
            ],
            &jan,
            Timeframe::MINUTE_1,
        );

        assert_eq!(out.cells.len(), 2);
        assert_ne!(
            out.cells[0].key, out.cells[1].key,
            "two strikes sharing a key is silent data loss, not a smaller feature"
        );
    }

    #[test]
    fn an_underlying_this_store_cannot_file_is_refused_once_and_not_once_per_month() {
        let months = [month(2025, 12), month(2026, 1)];
        let out = cells(
            &[found("BANK.BARODA", 29, 5_800_000)],
            &months,
            Timeframe::MINUTE_1,
        );

        assert!(
            out.cells.is_empty(),
            "no cell may be built from a name that cannot be filed"
        );
        assert_eq!(
            out.refused.len(),
            1,
            "one bad underlying, one error — not one per month"
        );
        let CellError::Underlying { ref name, .. } = out.refused[0];
        assert_eq!(name, "BANK.BARODA");
    }

    #[test]
    fn a_refusal_names_the_contract_and_says_its_bars_were_not_asked_for() {
        let out = cells(
            &[found("BANK.BARODA", 29, 5_800_000)],
            &[month(2026, 1)],
            Timeframe::MINUTE_1,
        );
        let said = out.refused[0].to_string();

        assert!(said.contains("BANK.BARODA"), "{said}");
        assert!(said.contains("were not asked for"), "{said}");
    }

    #[test]
    fn a_good_contract_beside_a_bad_one_still_produces_its_cell() {
        // The half that matters: a refusal must not take the rest of the month
        // down with it, or one unreadable name costs every other contract.
        let out = cells(
            &[
                found("BANK.BARODA", 29, 5_800_000),
                found("BANKNIFTY", 29, 5_800_000),
            ],
            &[month(2026, 1)],
            Timeframe::MINUTE_1,
        );

        assert_eq!(out.cells.len(), 1);
        assert_eq!(out.refused.len(), 1);
    }

    #[test]
    fn discovering_nothing_is_an_empty_result_and_not_an_error() {
        let out = cells(&[], &[month(2026, 1)], Timeframe::MINUTE_1);
        assert_eq!(out, Discovered::default());
    }

    #[test]
    fn a_cell_already_held_is_never_asked_for_again() {
        let out = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &[month(2026, 1)],
            Timeframe::DAY_1,
        );
        let mut held = HashSet::new();
        held.insert(out.cells[0].key);

        let work = gaps(&out.cells, &held);

        assert!(work.is_complete());
        assert_eq!(work.held, 1);
        assert_eq!(work.offered(), 1);
    }

    #[test]
    fn a_cell_not_held_is_missing_and_keeps_the_vendors_own_name() {
        let out = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &[month(2026, 1)],
            Timeframe::DAY_1,
        );
        let work = gaps(&out.cells, &HashSet::new());

        assert!(!work.is_complete());
        assert_eq!(work.held, 0);
        // The vendor's name is what goes back on the wire; a missing cell that
        // dropped it would have to re-render a name discovery already returned.
        assert_eq!(work.missing[0].vendor_symbol, "BANKNIFTY26JAN5800000CE");
    }

    #[test]
    fn a_half_held_month_reports_both_halves_and_they_sum() {
        let months = [month(2025, 12), month(2026, 1)];
        let out = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &months,
            Timeframe::MINUTE_1,
        );
        let mut held = HashSet::new();
        held.insert(out.cells[1].key);

        let work = gaps(&out.cells, &held);

        assert_eq!(work.missing.len(), 1);
        assert_eq!(work.held, 1);
        assert_eq!(
            work.offered(),
            out.cells.len(),
            "the two halves must sum to what was offered"
        );
        assert_eq!(work.missing[0].key.month, months[0]);
    }

    /// The probe form answers the same question as the set form.
    ///
    /// Two spellings of one question is two chances to disagree, so this pins
    /// them to each other on the same input rather than asserting each alone.
    #[test]
    fn the_probe_form_and_the_set_form_agree_cell_for_cell() {
        let months = [month(2025, 12), month(2026, 1)];
        let out = cells(
            &[found("BANKNIFTY", 29, 5_800_000)],
            &months,
            Timeframe::MINUTE_1,
        );
        let mut set = HashSet::new();
        set.insert(out.cells[1].key);

        let by_set = gaps(&out.cells, &set);
        let by_probe = gaps_by(&out.cells, |key| set.contains(key));

        assert_eq!(by_set, by_probe);
        assert_eq!(by_probe.held, 1);
        assert_eq!(by_probe.missing.len(), 1);
    }

    #[test]
    fn a_gap_over_nothing_is_complete_rather_than_unknown() {
        let work = gaps(&[], &HashSet::new());
        assert!(work.is_complete());
        assert_eq!(work.offered(), 0);
        assert_eq!(work, Work::default());
    }
}
