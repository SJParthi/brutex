#![cfg(test)]
//! BASE-COV-PULL (D-7332): the sentences of the crate's refusals no test had
//! read (Route B, recipe v4). A refusal's sentence is what an operator acts
//! on, so each states what happened and what was NOT done. Every value is
//! invented.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use crate::fetch::FetchError;
use crate::pricing::{PricingError, RateSource};
use crate::rolling::RollingError;
use crate::tenor::TenorError;

/// `said` holds every one of `words`.
fn says(said: &str, words: &[&str]) {
    for word in words {
        assert!(said.contains(word), "{word:?} is not in {said:?}");
    }
}

#[test]
fn a_rate_says_where_it_came_from() {
    use brutex_core::vendor::Vendor;
    assert_eq!(
        RateSource::Charter("a section").to_string(),
        "docs/00-charter.md: a section"
    );
    assert_eq!(
        RateSource::Operator.to_string(),
        "supplied by the operator with this request"
    );
    assert_eq!(
        RateSource::SolvedAgainst(Vendor::Dhan).to_string(),
        "solved against dhan's published implied volatility"
    );
}

#[test]
fn a_swept_underlying_has_a_slot_and_a_stranger_has_none() {
    let of = |word: &str| crate::pricing::slot_of(brutex_core::symbol::Symbol::new(word).unwrap());
    assert!(of("NIFTY").is_some());
    assert!(of("ZQNOSUCH").is_none());
}

#[test]
fn a_pricing_refusal_says_nothing_was_priced() {
    use crate::tenor::YearBasis;
    says(
        &PricingError::RateNotFinite { annual: f64::NAN }.to_string(),
        &["NaN", "not a finite number", "no contract was priced"],
    );
    says(
        &PricingError::RateImplausible {
            annual: f64::INFINITY,
        }
        .to_string(),
        &["inf", "a percentage entered where a decimal"],
    );
    says(
        &PricingError::RateUnsourced.to_string(),
        &["no source", "blank citation"],
    );
    says(
        &PricingError::BasisMismatch {
            rate: YearBasis::Calendar365,
            asked: YearBasis::Calendar365,
        }
        .to_string(),
        &["Calendar365", "Re-measure the rate"],
    );
    says(
        &PricingError::LadderUnknown {
            why: "an invented ladder".to_owned(),
        }
        .to_string(),
        &["nothing was priced: an invented ladder"],
    );
    says(
        &PricingError::NoSpotAtStamp {
            ts_micros: i64::MIN,
        }
        .to_string(),
        &[&i64::MIN.to_string(), "the previous minute's level"],
    );
}

#[test]
fn a_rolling_refusal_says_what_the_answer_lacked() {
    assert_eq!(
        RollingError::NotJson.to_string(),
        "the rolling answer is not JSON"
    );
    says(
        &RollingError::UnknownSide.to_string(),
        &["no response key", "the store is append-only"],
    );
    says(
        &RollingError::NoSide { key: "zq" }.to_string(),
        &["no `zq` object"],
    );
    says(
        &RollingError::NotAnArray { field: "zq" }.to_string(),
        &["`zq` is absent or is not an array"],
    );
    assert_eq!(
        RollingError::Unrepresentable { field: "zq" }.to_string(),
        "a `zq` value does not fit paisa as an i64"
    );
    says(
        &RollingError::Undecimal {
            field: "zq",
            text: "1e999".to_owned(),
        }
        .to_string(),
        &["`zq` cell holds 1e999", "not a decimal"],
    );
}

#[test]
fn the_rolling_url_is_the_base_and_each_path_segment() {
    let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
    else {
        panic!("Dhan is an HTTP broker");
    };
    let spec = dhan.fno.by_offset().unwrap();
    let url = crate::rolling::url(&spec, "https://invented.example");
    assert!(url.starts_with("https://invented.example/"), "{url}");
    assert_eq!(
        url.matches('/').count(),
        2 + spec.path.len(),
        "one slash for each segment: {url}"
    );
    let bare = crate::vendor::RollingSpec { path: &[], ..spec };
    assert_eq!(crate::rolling::url(&bare, "b"), "b");
}

#[test]
fn a_tenor_refusal_says_no_tenor_was_measured() {
    let expiry = brutex_core::instrument::Expiry::new(2026, 8, 27).unwrap();
    says(
        &TenorError::AlreadyExpired { seconds_past: 0 }.to_string(),
        &["0 second(s) at or past the close", "no time value left"],
    );
    says(
        &TenorError::ExpiryOffCalendar { expiry }.to_string(),
        &["2026-08-27", "No tenor was measured"],
    );
}

#[test]
fn a_fetch_refusal_says_nothing_was_sent() {
    let credential = |names_two, given_two| {
        FetchError::CredentialMismatch {
            names_two,
            given_two,
        }
        .to_string()
    };
    says(&credential(true, false), &["two", "one was"]);
    says(&credential(false, true), &["one", "two were"]);
    says(
        &FetchError::PathSegmentUnusable {
            placeholder: "{zq}",
            value: "a/b".to_owned(),
        }
        .to_string(),
        &["\"{zq}\"", "\"a/b\"", "Nothing was sent"],
    );
}

#[test]
fn a_membership_refusal_names_the_day_or_the_name() {
    use crate::fno_membership::{MembershipError as M, MembershipUnknown, UnknownWhy};
    use crate::session::Day;
    let on = Day::new(2024, 3, 1).unwrap();
    let later = Day::new(2024, 3, 2).unwrap();
    says(
        &MembershipUnknown {
            day: on,
            why: UnknownWhy::Empty,
        }
        .to_string(),
        &["2024-03-01", "no record is held"],
    );
    says(
        &MembershipUnknown {
            day: on,
            why: UnknownWhy::AfterSpan { known_to: later },
        }
        .to_string(),
        &["2024-03-01", "2024-03-02"],
    );
    assert_eq!(
        M::BadName { name: "x y".into() }.to_string(),
        "\"x y\" is not a symbol"
    );
    says(
        &M::NotAShare { name: "ZQ".into() }.to_string(),
        &["\"ZQ\"", "not a share"],
    );
    says(
        &M::Gap {
            from: later,
            last: on,
        }
        .to_string(),
        &["2024-03-02", "2024-03-01"],
    );
    says(
        &M::BadWindow {
            from: later,
            to: on,
        }
        .to_string(),
        &["2024-03-02", "2024-03-01"],
    );
    says(
        &M::TooManySpellings { day: on }.to_string(),
        &["2024-03-01"],
    );
    says(&M::SourceChanged { day: on }.to_string(), &["2024-03-01"]);
}

#[test]
fn a_run_lock_or_ledger_fault_says_who_holds_what() {
    use crate::gdfl_run::ledger::RunsLedgerFault as L;
    use crate::gdfl_run::locks::LockFault as F;
    let lock = || "imports/invented.lock".to_owned();
    assert_eq!(
        F::Refused {
            lock: lock(),
            why: "an invented refusal".to_owned(),
        }
        .to_string(),
        "run_lock_refused: imports/invented.lock: an invented refusal"
    );
    says(
        &F::Replaced { lock: lock() }.to_string(),
        &["imports/invented.lock"],
    );
    assert_eq!(
        F::Layout {
            why: "an invented layout".to_owned(),
        }
        .to_string(),
        "census_unreadable: the census layout cannot be read: an invented layout"
    );
    says(
        &L::Damaged { offset: u64::MAX }.to_string(),
        &[&u64::MAX.to_string(), "never repaired silently"],
    );
    says(
        &L::ClockImplausible {
            now: 0,
            why: "an invented reason".to_owned(),
        }
        .to_string(),
        &["the clock says 0", "an invented reason"],
    );
    says(
        &L::Lock(F::Replaced { lock: lock() }).to_string(),
        &["imports/invented.lock"],
    );
}

#[test]
fn a_stop_refusal_says_why_no_stop_was_written() {
    use crate::gdfl_run::stop::StopRefusal as S;
    assert_eq!(
        S::NotLive {
            asked: "42".to_owned(),
        }
        .to_string(),
        "run_not_live: no live GDFL run is 42"
    );
    says(
        &S::ClaimBeingWritten {
            lock: "a.lock".to_owned(),
        }
        .to_string(),
        &["a.lock", "ask again in a moment"],
    );
    says(
        &S::ByUnknown {
            by: "nobody".to_owned(),
        }
        .to_string(),
        &["by=nobody is not one of"],
    );
    says(
        &S::Unwritten {
            why: "an invented why".to_owned(),
        }
        .to_string(),
        &["an invented why"],
    );
}

#[test]
fn a_spool_refusal_names_its_path_and_record() {
    use crate::zerodha_verify::spool::SpoolError as S;
    let path = || std::path::PathBuf::from("/invented/spool");
    says(
        &S::Header {
            path: path(),
            why: "an invented header".to_owned(),
        }
        .to_string(),
        &["/invented/spool", "an invented header"],
    );
    says(
        &S::LogDamaged {
            path: path(),
            record: u64::MAX,
            why: "crc",
        }
        .to_string(),
        &["/invented/spool", &u64::MAX.to_string(), "crc"],
    );
    assert_eq!(
        S::Record {
            why: "an invented record".to_owned(),
        }
        .to_string(),
        "record_refused: an invented record"
    );
    says(
        &S::NotRegular { path: path() }.to_string(),
        &["/invented/spool"],
    );
}

#[test]
fn a_keep_refusal_names_its_word_and_what_was_not_written() {
    use crate::response_keep::{KeepError as K, Reason as R};
    says(
        &K::Path(store::path::PathError::EmptySegment { field: "symbol" }).to_string(),
        &["path_refused: "],
    );
    says(
        &K::LockStalled {
            lock: std::path::PathBuf::from("/invented/lock"),
            waited_ms: u64::MAX,
        }
        .to_string(),
        &["lock_stalled: /invented/lock", "nothing written"],
    );
    assert_eq!(
        K::NotKept {
            reason: R::LaterMonth,
        }
        .to_string(),
        "body_not_kept: later_month"
    );
}

#[test]
fn an_alert_refusal_says_its_code_and_the_ordinal() {
    use crate::alerts::AlertError as E;
    says(
        &E::NoSuchAlert { ordinal: 7 }.to_string(),
        &["no_such_alert", "7"],
    );
    assert_eq!(E::NotDamaged.to_string(), E::NotDamaged.code());
    assert_eq!(E::Full.to_string(), E::Full.code());
}

#[test]
fn a_discovery_refusal_says_what_was_not_reached() {
    use crate::chain::ChainError as R;
    let lookup = crate::fno::FnoError::RepeatedKey {
        key: "zq".to_owned(),
    };
    assert_eq!(R::Lookup(lookup.clone()).to_string(), lookup.to_string());
    assert_eq!(
        R::Transport {
            url: "https://invented.example".to_owned(),
            why: "refused".to_owned(),
        }
        .to_string(),
        "https://invented.example was not reached: refused"
    );
}

#[test]
fn every_rung_has_its_page_word_and_no_two_share_one() {
    use crate::vendor::Granularity as G;
    let rungs = [
        (G::Tick, "Tick"),
        (G::Second1, "1 second"),
        (G::Second5, "5 seconds"),
        (G::Minute1, "1 minute"),
        (G::Minute3, "3 minutes"),
        (G::Minute5, "5 minutes"),
        (G::Minute15, "15 minutes"),
        (G::Minute30, "30 minutes"),
        (G::Hour1, "1 hour"),
        (G::Day1, "1 day"),
        (G::Week1, "1 week"),
    ];
    let mut seen = std::collections::HashSet::new();
    for (rung, word) in rungs {
        assert_eq!(rung.label(), word);
        assert!(seen.insert(word), "{word} is two rungs");
    }
}

/// The options tree's names: a folder is an underlying, a stray file is not,
/// a tree not made yet holds none, and a tree that is a file is refused.
#[test]
fn the_options_tree_names_its_folders_and_only_those() {
    use crate::gdfl_run::scope::option_underlyings;
    let root = crate::gdfl_fixtures::scratch("cov-underlyings");
    assert_eq!(option_underlyings(&root).unwrap(), Vec::<String>::new());
    let tree = root.join("bars/gdfl/NSE/FNO");
    std::fs::create_dir_all(tree.join("ZQB")).unwrap();
    std::fs::create_dir_all(tree.join("ZQA")).unwrap();
    std::fs::write(tree.join("a-file"), b"x").unwrap();
    assert_eq!(option_underlyings(&root).unwrap(), ["ZQA", "ZQB"]);
    let other = crate::gdfl_fixtures::scratch("cov-underlyings-file");
    std::fs::create_dir_all(other.join("bars/gdfl/NSE")).unwrap();
    std::fs::write(other.join("bars/gdfl/NSE/FNO"), b"a file").unwrap();
    assert!(option_underlyings(&other).is_err());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(other).unwrap();
}

/// `--stored` names what the scope adds: sorted, once each, for each kind,
/// and an options run adds nothing.
#[test]
fn the_stored_filter_is_sorted_and_names_each_once() {
    use crate::gdfl_import::ImportKind;
    use crate::gdfl_run::scope::stored_filter;
    let scope = crate::gdfl_cm::CmScope::swept();
    for kind in [ImportKind::Indices, ImportKind::Stocks, ImportKind::Options] {
        let names = stored_filter(kind, &scope);
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
    assert_eq!(
        stored_filter(ImportKind::Options, &scope),
        Vec::<String>::new()
    );
}

#[test]
fn a_day_not_owed_says_which_document_says_so() {
    use crate::gaps::NotOwed;
    use crate::session::Day;
    let on = Day::new(2024, 3, 1).unwrap();
    says(
        &NotOwed::BeforeListing { listed_on: on }.evidence(),
        &["before its listing on 2024-03-01", "NSE security master"],
    );
    says(
        &NotOwed::AfterDelisting { delisted_on: on }.evidence(),
        &["after its removal on 2024-03-01"],
    );
    assert_eq!(
        NotOwed::Unread.evidence(),
        "no stored reading covers the day"
    );
}

#[test]
fn an_axis_says_its_own_word() {
    use crate::refusal::Axis;
    for axis in [Axis::Status, Axis::ErrorName] {
        assert_eq!(axis.to_string(), axis.word());
    }
}

/// A folder of no day zips answers its own root and takes the scope it is
/// given.
#[test]
fn a_day_zips_source_keeps_its_root_and_takes_its_scope() {
    use crate::gdfl_dayzips::DayZips;
    let root = crate::gdfl_fixtures::scratch("cov-dayzips");
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(loose.root(), root);
    let scope = std::sync::Arc::new(crate::gdfl_cm::CmScope::swept());
    let scoped = loose.with_scope(scope.clone());
    assert!(std::sync::Arc::ptr_eq(
        &crate::gdfl_cm::CmSource::scope(&scoped),
        &scope
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_contract_is_asked_as_a_derivative_for_the_window_given() {
    use crate::vendor::{Granularity, Listing};
    let (from, to) = (
        crate::session::Day::new(2024, 3, 1).unwrap(),
        crate::session::Day::new(2024, 3, 2).unwrap(),
    );
    let window = crate::session::Window::new(from, to).unwrap();
    let found = crate::fno::Found {
        vendor_symbol: "ZQ24MARFUT".to_owned(),
        underlying: "ZQ".to_owned(),
        contract: brutex_core::instrument::Contract::parse("2024-03-28-FUT").unwrap(),
        expiry: brutex_core::instrument::Expiry::new(2024, 3, 28).unwrap(),
        option: None,
    };
    let asked = crate::chain::request(&found, window, Granularity::Day1);
    assert_eq!(asked.instrument_id, "ZQ24MARFUT");
    assert_eq!(asked.listing, Listing::Derivative);
    assert_eq!(asked.window, window);
}
