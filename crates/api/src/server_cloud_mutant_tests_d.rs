//! Cloud mutation pass of 17 Oct (`cloud/mut-api-b-20261017`) over
//! `server.rs`: lines and branch sides that a coverage run of the whole `api`
//! lib suite on `feat/db-fix` `f9cdb584`, with the 16 Oct pass re-applied,
//! left unexecuted, each driven where a test can reach it.
//!
//! No test here opens a socket to anything but `127.0.0.1`, reads a
//! credential, or contacts a vendor. Every instrument and every day is
//! invented for the fixture.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a failed fixture must fail the test by name"
)]

use super::*;
use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind};

/// A fresh, empty directory under this process's scratch prefix.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("cloudd-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// One full-session NSE day on or after the 2026-08-03 cash-auction change,
/// asked of the calendar rather than assumed.
fn auction_era_open_day() -> Day {
    let start = Day::new(2026, 8, 3).expect("a real date").days_from_epoch();
    (start..start + 30)
        .filter_map(|n| Day::from_days(n).ok())
        .find(|day| {
            pull::calendar::kind_of(i64::from(day.days_from_epoch()))
                == pull::calendar::DayKind::Open(pull::calendar::Session::full())
        })
        .expect("a full session within thirty days of the change")
}

/// A read over `kept` (one vendor's listings) with nothing else wrong.
fn read_of(vendor: Vendor, kept: Vec<brutex_core::vendor::Listing>) -> Read {
    let merged = crate::merge::merge(&[crate::merge::Source {
        vendor,
        kept,
        declined: Vec::new(),
    }]);
    Read::new(merged, Vec::new(), Vec::new(), 0, 0, 0)
}

/// **A read whose only fault is one decline nobody chose is not clean.**
/// No row is unreadable, no vendor is unavailable and no listing class is
/// unrecognised, so only `non_routine` is left to refuse it. Covers the false
/// side of `Read::is_clean`'s `non_routine == 0` (server.rs:564), which every
/// other unclean read short-circuited before.
#[test]
fn a_non_routine_decline_alone_makes_the_read_unclean() {
    let read = |non_routine| {
        Read::new(
            crate::merge::Merged::default(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            non_routine,
        )
    };
    let clean = read(0);
    assert!(clean.is_clean(), "the premise: nothing else is wrong");
    assert_eq!(clean.status(), "ok");
    let declined = read(1);
    assert!(!declined.is_clean());
    assert_eq!(declined.status(), "DEGRADED");
}

/// **An NSE cash series that names a contract has no dated cash audit.** The
/// guard answers before any ISIN is looked up, so a name with no ISIN is not
/// refused for it. Covers the third disqualifier of
/// `audit_cash_schedule_window`'s guard (server.rs:1520); the other two are
/// `only_an_nse_cash_series_with_no_contract_has_a_dated_cash_audit`'s.
#[tokio::test]
async fn an_nse_cash_contract_has_no_dated_cash_audit() {
    let dir = fresh_dir("cash-contract-masters");
    let root = fresh_dir("cash-contract-store");
    let site = Site::load(&dir, &root);
    let day = auction_era_open_day();
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    let asked = Addressed {
        vendor: Vendor::Zerodha,
        month: day.year_month().expect("a month"),
        timeframe: store::path::Timeframe::MINUTE_1,
        contract: Some(
            brutex_core::instrument::Contract::parse("2024-03-07-2200000-CE").expect("a contract"),
        ),
        exchange: "NSE".to_owned(),
        segment: "CASH".to_owned(),
        symbol: "ZZNOISIN".to_owned(),
    };
    let answered = audit_cash_schedule_window(&site, &asked, window).await;
    assert!(
        matches!(answered, Ok(None)),
        "{:?}",
        answered.map(|schedule| schedule.is_some())
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A run whose store refused a write, with no member failure, is classed
/// as the store's.** Covers the right side of `retry_fact`'s `&&`
/// (server.rs:4770): the store fact alone is enough to class a run.
#[test]
fn a_run_the_store_refused_with_no_member_failure_is_classed_store() {
    use crate::pullrun::RetryClass;
    let mut run = BrokerRun::default();
    assert_eq!(run.retry_fact(), None, "the premise: nothing to class");
    run.store_unavailable = Some("the store will not take a write".to_owned());
    let (label, said) = run.retry_fact().expect("a store fault is classed");
    assert_eq!(label, crate::pullrun::RETRY_CLASS_FACT);
    assert!(
        said.starts_with(&format!("{} — 0 failure(s)", RetryClass::Store.word())),
        "{said}"
    );
    let page = stored_html("Spot pull", "FAILED", "r", &[(label, said)]);
    assert_eq!(RetryClass::of_receipt(&page), Some(RetryClass::Store));
}

/// **A contract key is not a spot instrument, whatever id it carries.** The
/// key resolves to a vendor id, so the refusal is the kind's and nothing
/// else's. Covers `spot_vendor_identity_for`'s "not a spot instrument"
/// refusal (server.rs:5988-5989).
#[test]
fn a_contract_key_is_not_a_spot_instrument() {
    let mut key = InstrumentKey::cash(Exchange::Nse, "TCS").expect("a key");
    key.kind = Kind::Future {
        expiry: Expiry::new(2025, 5, 29).expect("an expiry"),
    };
    let read = read_of(
        Vendor::Dhan,
        vec![brutex_core::vendor::Listing {
            vendor_id: brutex_core::vendor::VendorId::new("4242").expect("an id"),
            key,
            isin: None,
            unsuffixed: None,
        }],
    );
    assert!(
        read.merged.by_key.get(&key).is_some(),
        "the premise: the key is in the universe"
    );
    let why = spot_vendor_identity_for(&read, &key, Vendor::Dhan, ingest::CashIdentity::Isin)
        .expect_err("a future is not spot");
    assert_eq!(why, "TCS: not a spot instrument");
}

/// One NSE cash minute window of `instrument`, as `broker_window` lands it.
fn landed_cash(instrument: &str) -> BrokerWindow {
    let spec = match pull::vendor::Feed::Dhan.descriptor().transport {
        pull::vendor::Transport::Http(spec) => Some(spec),
        pull::vendor::Transport::LocalArchive(_) => None,
    }
    .expect("Dhan is an HTTP feed");
    let day = auction_era_open_day();
    BrokerWindow {
        listing: pull::vendor::Listing::Equity,
        contract: None,
        unfetched: None,
        before_first_bar: None,
        instrument: instrument.to_owned(),
        origin: "test only".to_owned(),
        spec,
        exchange: "NSE",
        segment: "CASH",
        store_vendor: Vendor::Dhan,
        window: pull::session::Window::new(day, day).expect("a one-day window"),
        granularity: pull::vendor::Granularity::Minute1,
        bodies: Vec::new(),
    }
}

/// **A cash schedule is refused for any source that is not the exact NSE
/// cash identity of the instrument asked**, before any body is read: another
/// exchange, another segment, a contract, or another name. A daily window
/// and an index listing owe none at all. Covers each disqualifier of
/// `prepare_cash_schedule`'s identity guard (server.rs:5644-5647).
#[tokio::test]
async fn a_cash_schedule_is_refused_for_any_source_but_the_exact_nse_cash_identity() {
    let root = fresh_dir("cash-schedule-identity");
    let site = Site::load(&root, &root);
    let key = InstrumentKey::cash(Exchange::Nse, "TCS").expect("a key");
    let refused = "cash schedule requires the exact NSE cash source identity";
    let mut bse = landed_cash("TCS");
    bse.exchange = "BSE";
    let mut index = landed_cash("TCS");
    index.segment = "INDEX";
    let mut contract = landed_cash("TCS");
    contract.contract = brutex_core::instrument::Contract::of(Kind::Future {
        expiry: Expiry::new(2026, 8, 27).expect("an expiry"),
    });
    assert!(contract.contract.is_some(), "the premise: a contract");
    for (why, landed) in [
        ("another exchange", bse),
        ("another segment", index),
        ("a contract", contract),
        ("another name", landed_cash("INFY")),
    ] {
        let mut dated = std::collections::HashMap::new();
        let answered = prepare_cash_schedule(&landed, &[], &key, &site, &mut dated).await;
        assert_eq!(
            answered.map(|schedule| schedule.is_some()),
            Err(refused.to_owned()),
            "{why}"
        );
    }
    let mut daily = landed_cash("TCS");
    daily.granularity = pull::vendor::Granularity::Day1;
    let mut listed_index = landed_cash("TCS");
    listed_index.listing = pull::vendor::Listing::Index;
    for (why, landed) in [
        ("a daily window", daily),
        ("an index listing", listed_index),
    ] {
        let mut dated = std::collections::HashMap::new();
        let answered = prepare_cash_schedule(&landed, &[], &key, &site, &mut dated).await;
        assert_eq!(
            answered.map(|schedule| schedule.is_some()),
            Ok(false),
            "{why}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}
