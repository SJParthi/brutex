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
/// let cell = ContractCell { key, vendor_symbol: "BANKNIFTY26JAN58000CE".to_owned() };
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
