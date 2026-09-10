//! Bounded selector/projection refusal tests; no market computation is fabricated.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded fixtures and executable assertions"
)]
use super::*;
const ID: &str = "1212121212121212121212121212121212121212121212121212121212121212";
#[test]
fn search_selectors_require_exact_parent_and_child_pins_for_continuations() {
    assert!(Asked::parse(&format!("identity={ID}")).is_ok());
    assert!(Asked::parse(&format!("identity={ID}&pin={ID}&batch=0&rung=7")).is_ok());
    assert!(Asked::parse(&format!("identity={ID}&pin={ID}&batch=18446744073709551615&rung=0&completion={ID}&offset=16&limit=256")).is_ok());
    for suffix in [
        "&batch=0&rung=0",
        "&pin=short",
        "&pin=x&pin=y",
        "&path=/tmp",
        "&limit=16",
        "&pin=1212&batch=0&rung=8",
        "&batch=01",
        "&offset=0",
    ] {
        assert!(
            Asked::parse(&format!("identity={ID}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    assert!(Asked::parse(&format!("identity={ID}&pin={ID}&batch=0&rung=0&offset=1")).is_err());
}
#[test]
fn reserved_paused_refused_complete_and_exhausted_are_distinct() {
    assert_eq!(phase(0, true, false).unwrap(), ("reserved", "running"));
    assert_eq!(phase(0, false, false).unwrap(), ("reserved", "paused"));
    assert_eq!(phase(1, false, false).unwrap(), ("complete", "paused"));
    assert_eq!(phase(1, true, true).unwrap(), ("complete", "completed"));
    assert_eq!(phase(2, true, false).unwrap(), ("refused", "refused"));
    assert!(phase(0, true, true).is_err());
    assert!(phase(9, false, false).is_err());
}
#[test]
fn complete_summary_counts_and_zero_links_refuse_inconsistent_projections() {
    let mut row = SearchSummary {
        child: cli::boolean_evidence::QualifiedLink {
            identity: [0; 32],
            pin: [0; 32],
        },
        count: 0,
        counts: [0; 4],
        projection: [0; 32],
        allocation: [0; 32],
    };
    assert!(summary(0, &row).unwrap()["qualification"].is_null());
    row.child.identity = [1; 32];
    assert!(summary(0, &row).is_err());
    row.child.pin = [2; 32];
    row.projection = [3; 32];
    row.allocation = [4; 32];
    row.count = u64::MAX;
    row.counts = [u64::MAX, 0, 0, 0];
    assert_eq!(summary(7, &row).unwrap()["count"], "18446744073709551615");
    row.counts[1] = 1;
    assert!(summary(0, &row).is_err());
    assert!(summary(8, &row).is_err());
}
#[test]
fn every_selected_summary_keeps_its_original_physical_rung_and_count() {
    let summaries = std::array::from_fn(|index| SearchSummary {
        child: cli::boolean_evidence::QualifiedLink {
            identity: [1; 32],
            pin: [2; 32],
        },
        count: index as u64 + 1,
        counts: [0, 0, 0, index as u64 + 1],
        projection: [3; 32],
        allocation: [4; 32],
    });
    for mask in 1_u16..256 {
        let labels: Vec<_> = cli::EVERY_RUNG
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, rung)| *rung)
            .collect();
        let scope = cli::boolean_campaign::RungScope::new(&labels).unwrap();
        let rows = selected_summaries(&summaries, scope).unwrap();
        assert_eq!(rows.len(), labels.len());
        for (row, index) in rows.iter().zip(scope.indices()) {
            assert_eq!(row["rung"], cli::EVERY_RUNG[index]);
            assert_eq!(row["rung_index"], index.to_string());
            assert_eq!(row["count"], (index + 1).to_string());
        }
    }
}
#[test]
fn missing_search_names_authoritative_root_and_creates_nothing() {
    let root = crate::scratch::path("missing-qualified-search");
    let asked = Asked::parse(&format!("identity={ID}")).unwrap();
    let why = render(&root, &asked, 67_108_864, 1000).unwrap_err();
    assert!(why.contains(root.to_str().unwrap()));
    assert!(why.contains("BRUTEX_STORE must match"));
    assert!(!root.exists());
}
#[tokio::test]
async fn invalid_public_search_queries_refuse_before_reading_config_or_sources() {
    for uri in [
        "/boolean-qualified-search.json",
        "/boolean-qualified-search.json?identity=x",
        "/boolean-qualified-search.json?path=/tmp",
    ] {
        let out = search_json(uri.parse().unwrap()).await;
        assert_eq!(out.0, StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_str(&out.2).unwrap();
        assert_eq!(body["rows"], json!([]));
        assert_eq!(body["status"], "refused");
    }
}
