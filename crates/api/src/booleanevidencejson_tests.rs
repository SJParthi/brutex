#![cfg(test)]
//! Finite selector/projection tests. These do not price or attest market bars.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded projection fixtures and assertions"
)]
use super::*;

const ID: &str = "1212121212121212121212121212121212121212121212121212121212121212";

#[test]
fn exact_model_selectors_require_completion_for_every_later_or_detail_page() {
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        assert!(Asked::parse(&format!("identity={ID}"), model).is_ok());
        assert!(
            Asked::parse(
                &format!("identity={ID}&completion={ID}&offset=1&limit=256"),
                model
            )
            .is_ok()
        );
        for suffix in [
            "&offset=1",
            "&offset=01",
            "&limit=0",
            "&limit=257",
            "&path=/tmp",
            "&identity=x",
            "&completion=",
            "&offset=18446744073709551616",
            "&kind=trades",
            "&limit=2&limit=2",
            "&max_bytes=1610612736",
        ] {
            assert!(
                Asked::parse(&format!("identity={ID}{suffix}"), model).is_err(),
                "{model:?}: {suffix}"
            );
        }
    }
    for kind in ["sources", "splits"] {
        assert!(Asked::parse(&format!("identity={ID}&kind={kind}"), Model::Statistics).is_err());
        let query = format!("identity={ID}&completion={ID}&kind={kind}");
        assert!(Asked::parse(&query, Model::Statistics).is_ok());
        assert!(Asked::parse(&query, Model::Admission).is_err());
        assert!(Asked::parse(&query, Model::Qualification).is_err());
    }
}

#[test]
fn page_cardinality_and_completion_never_accept_a_partial_or_replacement_page() {
    let asked = Asked::parse(
        &format!("identity={ID}&completion={ID}&offset=1&limit=2"),
        Model::Statistics,
    )
    .unwrap();
    let body = base(
        &asked,
        [0x12; 32],
        4,
        vec![json!({"index":"1"}), json!({"index":"2"})],
    )
    .unwrap();
    assert_eq!(body["next"], "3");
    assert_eq!(body["total"], "4");
    assert!(base(&asked, [0x13; 32], 4, vec![json!({}), json!({})]).is_err());
    assert!(base(&asked, [0x12; 32], 4, vec![json!({})]).is_err());
    assert!(base(&asked, [0x12; 32], 0, vec![]).is_err());
    assert_eq!(
        base(&asked, [0x12; 32], 1, vec![]).unwrap()["next"],
        Value::Null
    );
}

#[test]
fn exact_float_bits_and_unavailable_codes_are_not_rounded_into_success() {
    for value in [0.0_f64, -0.0, f64::MIN_POSITIVE, f64::MAX] {
        let projected = float(value.to_bits());
        assert_eq!(projected["bits"], value.to_bits().to_string());
        let restored = projected["decimal"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert_eq!(restored.to_bits(), value.to_bits());
    }
    for bits in [
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0042,
    ] {
        let projected = float(bits);
        assert_eq!(projected["bits"], bits.to_string());
        assert!(projected["decimal"].is_null());
    }
    assert_eq!(
        availability(2).unwrap(),
        "unavailable-constant-return-candidate"
    );
    assert_eq!(availability(3).unwrap(), "unavailable-numerical-refusal");
    assert!(availability(0).is_err());
    assert!(availability(4).is_err());
    let test = family_test([0, 0, 9_007_199_254_740_993, u64::MAX, 1, 2, 3, 4]);
    assert_eq!(test["exact_probability"]["numerator"], "9007199254740993");
    assert_eq!(
        test["exact_probability"]["denominator"],
        u64::MAX.to_string()
    );
}

#[tokio::test]
async fn public_evidence_routes_reject_invalid_selectors_before_accessing_a_store() {
    for uri in [
        "/boolean-statistics.json?identity=bad",
        "/boolean-statistics.json?kind=sources",
        "/boolean-statistics.json?identity=12&path=/tmp",
    ] {
        let reply = statistics_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_str(&reply.2).unwrap();
        assert_eq!(body["status"], "refused");
        assert_eq!(body["rows"], json!([]));
    }
    let reply = admission_json(
        format!("/boolean-admission.json?identity={ID}&completion={ID}&kind=splits")
            .parse()
            .unwrap(),
    )
    .await;
    assert_eq!(reply.0, StatusCode::BAD_REQUEST);
    let reply = qualification_json(
        format!("/boolean-qualification.json?identity={ID}&offset=1")
            .parse()
            .unwrap(),
    )
    .await;
    assert_eq!(reply.0, StatusCode::BAD_REQUEST);
}

#[test]
fn missing_saved_receipt_names_configured_root_without_creating_or_searching_it() {
    let root = crate::scratch::path("boolean-observer-no-evidence");
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        let asked = Asked::parse(&format!("identity={ID}"), model).unwrap();
        let why = render(&root, &asked).unwrap_err();
        assert!(why.contains(&root.display().to_string()));
        assert!(why.contains("OUTPUT_ROOT"));
        assert!(why.contains("no other folder searched"));
        assert!(!root.exists());
    }
}

#[test]
fn every_evidence_model_names_the_exact_server_owned_budget_on_authentication_refusal() {
    let root = crate::scratch::path("boolean-observer-exact-budget");
    let budget = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new(
        "1610612736",
    )))
    .unwrap();
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        let asked = Asked::parse(&format!("identity={ID}"), model).unwrap();
        let why = render_with_budget(&root, &asked, budget).unwrap_err();
        assert!(
            why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
        );
        assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
    }
    assert!(!root.exists());
}

#[test]
fn daily_and_weekly_pages_require_exact_setting_parent_pin_and_bounded_period_selector() {
    for kind in ["index-days", "index-weeks"] {
        for period in ["full", "training", "later"] {
            let query = format!(
                "identity={ID}&completion={ID}&kind={kind}&setting=7&period={period}&offset=2&limit=256"
            );
            let asked = Asked::parse(&query, Model::Qualification).unwrap();
            assert_eq!(asked.setting, Some(7));
            assert_eq!(asked.period, period);
            assert_eq!(asked.offset, 2);
            assert_eq!(asked.limit, 256);
            assert!(Asked::parse(&query, Model::Admission).is_err());
        }
        for invalid in [
            format!("identity={ID}&kind={kind}&setting=0"),
            format!("identity={ID}&completion={ID}&kind={kind}"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&period=unknown"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&setting=1"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&limit=257"),
        ] {
            assert!(
                Asked::parse(&invalid, Model::Qualification).is_err(),
                "{invalid}"
            );
        }
    }
    assert!(Asked::parse(&format!("identity={ID}&setting=0"), Model::Qualification).is_err());
    assert!(Asked::parse(&format!("identity={ID}&period=full"), Model::Qualification).is_err());
}
