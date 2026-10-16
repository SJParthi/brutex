//! Mutant-killing tests for the public items of the GDFL import's readers,
//! the fold, the scope ledger and the scrub (cloud mutation round,
//! 2026-10-16; `reports/cloud-mut-pull-20261016.md`).
//!
//! Each test names the behaviour one or more surviving `cargo mutants`
//! mutants broke and asserts exactly that behaviour, so the mutant fails it.
//! Private items are tested beside their code, in the `cloud_mutant_tests`
//! module at the end of each source file.
//!
//! # Every value here is invented
//!
//! The repository is public and the vendor's licence is UNVERIFIED: no row,
//! price or file of any vendor appears here. Tickers are real index and
//! share names only where the code under test admits nothing else (the swept
//! surface); every price, quantity and stamp is made up.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use brutex_core::instrument::{Contract, Exchange, Expiry, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey};
use store::path::{Timeframe, YearMonth};

/// A census entry of `contract` with one bar.
fn entry_of(contract: Option<Contract>) -> Entry {
    Entry {
        key: EntryKey {
            contract,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").unwrap(),
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2031, 3).unwrap(),
        },
        rows: 1,
        first_ts_micros: 1,
        last_ts_micros: 1,
    }
}

// ═══ scrub ════════════════════════════════════════════════════════════════

/// `scrub::contract_of` hands back the entry's own contract, and `None` for
/// spot (kills `-> None` and, where it compiles, `-> Some(Default)`).
#[test]
fn scrub_contract_of_is_the_entrys_own_contract() {
    let contract = Contract::of(Kind::Option {
        expiry: Expiry::new(2031, 3, 27).unwrap(),
        strike: Paisa::from_raw(1_234_500),
        side: OptionSide::Put,
    })
    .unwrap();
    assert_eq!(
        pull::scrub::contract_of(&entry_of(Some(contract))),
        Some(contract)
    );
    assert_eq!(pull::scrub::contract_of(&entry_of(None)), None);
}
