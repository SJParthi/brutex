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
        &PricingError::NoSpotAtStamp { ts_micros: i64::MIN }.to_string(),
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
