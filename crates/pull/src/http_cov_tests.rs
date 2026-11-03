#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): the arms of `http` no test had reached
//! (Route B on `c9d653d5f`, `state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt`):
//! a credential's `Debug`, a rate budget the governor refuses, a source a
//! caller left ungoverned, and a discovery field in a bars request. The
//! descriptor is the tests' invented one (`https://vendor.invalid`); no
//! request leaves the process.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::tests::spec_top;
use super::*;

/// A credential's `Debug` names each secret it holds and shows neither: one
/// secret, and a pair whose second is shown held, never its value.
#[test]
fn a_credential_says_what_it_holds_and_never_the_value() {
    let one = format!("{:?}", Credential::token("zq-token-invented".to_owned()));
    assert_eq!(one, r#"Credential { token: "<redacted>", key: None }"#);
    let pair = format!(
        "{:?}",
        Credential::pair("zq-key-invented".to_owned(), "zq-token-invented".to_owned())
    );
    assert_eq!(
        pair,
        r#"Credential { token: "<redacted>", key: Some("<redacted>") }"#
    );
}

/// A descriptor whose budget the governor will not take (a ceiling of zero,
/// and one past its range at once) is refused when the source is built,
/// naming the descriptor, and nothing is sent: an unenforced budget never
/// reads as an enforced one.
#[test]
fn a_budget_no_governor_takes_is_refused_when_the_source_is_built() {
    for budget in [
        crate::vendor::Budget {
            per_second: Some(0),
            per_minute: None,
            per_day: None,
        },
        crate::vendor::Budget {
            per_second: Some(u32::MAX),
            per_minute: Some(0),
            per_day: Some(u32::MAX),
        },
    ] {
        let spec = HttpSpec {
            budget,
            ..spec_top(PriceScale::Paisa)
        };
        let refused = HttpSource::new(spec, Credential::token("zq".to_owned())).unwrap_err();
        let FetchError::TransportFailed { detail } = refused else {
            panic!("{refused:?}");
        };
        assert!(
            detail.contains(
                "https://vendor.invalid declares a rate budget this build cannot enforce"
            ),
            "{detail}"
        );
    }
}

/// A source a caller shared with no governor is ungoverned from then on: a
/// governor handed to it later is not taken (it would enforce a ceiling the
/// caller never shared), and a permit is waited for no time at all.
#[test]
fn a_source_shared_with_no_governor_stays_ungoverned() {
    let source = HttpSource::new(
        spec_top(PriceScale::Paisa),
        Credential::token("zq".to_owned()),
    )
    .unwrap();
    assert!(
        source.governor().is_some(),
        "a declared budget, even one of no span, is governed"
    );
    let shared = source.sharing(None);
    assert!(shared.governor().is_none());
    let offered = crate::rate::Governor::new(Some(1), None, None).unwrap();
    let again = shared.sharing(Some(std::sync::Arc::new(std::sync::Mutex::new(offered))));
    assert!(again.governor().is_none(), "never governed after the fact");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    assert_eq!(runtime.block_on(again.wait_for_permit()), Ok(()));
}

/// Each discovery field (underlying, year, month, expiry date) in a bars
/// request is refused by its name, never sent with some other value.
#[test]
fn a_discovery_field_in_a_bars_request_is_refused_by_name() {
    let source = HttpSource::new(
        spec_top(PriceScale::Paisa),
        Credential::token("zq".to_owned()),
    )
    .unwrap();
    let day = crate::session::Day::new(2024, 4, 1).unwrap();
    let request = BarRequest {
        instrument_id: "13".to_owned(),
        listing: crate::vendor::Listing::Index,
        window: crate::session::Window::new(day, day).unwrap(),
        granularity: crate::vendor::Granularity::Minute1,
    };
    for (name, value) in [
        ("underlying", crate::vendor::ParamValue::Underlying),
        ("year", crate::vendor::ParamValue::Year),
        ("month", crate::vendor::ParamValue::Month),
        ("expiry", crate::vendor::ParamValue::ExpiryDate),
    ] {
        let param = crate::vendor::Param { name, value };
        let refused = source
            .resolve_param(&param, &request, "2024-04-01", "2024-04-02")
            .unwrap_err();
        let FetchError::NotABarsParam { field } = refused else {
            panic!("{refused:?}");
        };
        assert_eq!(field, name);
    }
}
