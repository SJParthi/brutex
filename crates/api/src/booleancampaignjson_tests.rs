//! Finite selector and projection tests; these are not generated market runs.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded fixtures and assertions"
)]
use super::*;
use cli::boolean_campaign::{Catalog, Status};

const ID: &str = "1212121212121212121212121212121212121212121212121212121212121212";

#[test]
fn exact_campaign_selectors_refuse_paths_history_and_repeated_or_malformed_pins() {
    assert!(Asked::parse(&format!("identity={ID}")).is_ok());
    assert!(Asked::parse(&format!("identity={ID}&pin={ID}")).is_ok());
    for suffix in [
        "&pin=",
        "&pin=short",
        "&pin=x&pin=y",
        "&path=/tmp",
        "&sequence=0",
        "&limit=8",
        "&identity=x",
        "&completion=x",
    ] {
        assert!(
            Asked::parse(&format!("identity={ID}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    assert!(Asked::parse("").is_err());
    assert!(Asked::parse(&"x".repeat(513)).is_err());
}

#[test]
fn missing_or_replaced_pin_never_falls_back_to_a_new_campaign_snapshot() {
    assert!(require_pin(None, [3; 32]).is_ok());
    assert!(require_pin(Some([3; 32]), [3; 32]).is_ok());
    assert!(
        require_pin(Some([2; 32]), [3; 32])
            .unwrap_err()
            .contains("explicitly refresh")
    );
}

#[test]
fn all_recorded_states_preserve_expected_uncompleted_families_and_exact_links() {
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let family = cli::boolean_observation::ResearchFamilyV1::new(key).unwrap();
    let states = [
        Status::Waiting,
        Status::Running,
        Status::Paused,
        Status::Refused,
        Status::Completed,
    ];
    let mut input = Vec::new();
    for status in states {
        input.push(Rung {
            rung: "1min",
            status,
            reason: if status == Status::Refused {
                "exact source changed".into()
            } else {
                String::new()
            },
            catalogs: vec![Catalog {
                family,
                expected: [3; 32],
                completion: (status == Status::Completed).then_some([4; 32]),
            }],
            statistics: (status == Status::Completed).then_some(Link {
                identity: [5; 32],
                completion: [6; 32],
            }),
            admission: (status == Status::Completed).then_some(Link {
                identity: [7; 32],
                completion: [8; 32],
            }),
        });
    }
    let projected = rows(&input);
    for (index, status) in states.iter().enumerate() {
        assert_eq!(projected[index]["state"], status.as_str());
        assert_eq!(
            projected[index]["expected_catalogs"][0]["identity"],
            crate::server::hex32([3; 32])
        );
        assert_eq!(
            projected[index]["catalogs"].as_array().unwrap().len(),
            usize::from(*status == Status::Completed)
        );
    }
    assert!(projected[0]["expected_catalogs"][0]["completion"].is_null());
    assert!(projected[0]["admission"].is_null());
    assert_eq!(projected[3]["reason"], "exact source changed");
    assert_eq!(
        projected[4]["catalogs"][0]["completion"],
        crate::server::hex32([4; 32])
    );
    assert_eq!(
        projected[4]["statistics"]["completion"],
        crate::server::hex32([6; 32])
    );
    assert_eq!(
        projected[4]["admission"]["completion"],
        crate::server::hex32([8; 32])
    );
}

#[tokio::test]
async fn public_campaign_route_refuses_invalid_selectors_before_reading_a_root() {
    for uri in [
        "/boolean-campaign.json",
        "/boolean-campaign.json?identity=short",
        "/boolean-campaign.json?path=/tmp",
    ] {
        let reply = campaign_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_str(&reply.2).unwrap();
        assert_eq!(body["status"], "refused");
        assert_eq!(body["rows"], json!([]));
        let reply = qualified_campaign_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
    }
}

#[test]
fn missing_campaign_reports_configured_root_without_creating_or_searching_outputs() {
    let root = crate::scratch::path("missing-boolean-campaign");
    let why = render(
        &root,
        &Asked {
            identity: [0x12; 32],
            pin: None,
        },
    )
    .unwrap_err();
    assert!(why.contains(root.to_str().unwrap()));
    assert!(why.contains("BRUTEX_STORE must match command OUTPUT_ROOT"));
    assert!(!root.exists());
    let why = render_qualified(
        &root,
        &Asked {
            identity: [0x12; 32],
            pin: None,
        },
    )
    .unwrap_err();
    assert!(why.contains(root.to_str().unwrap()));
    assert!(!root.exists());
}

#[test]
fn qualified_overview_authenticates_all_eight_initial_units_and_refuses_replacement() {
    use std::fs;
    let root = crate::scratch::path("qualified-eight-projection");
    let descriptor = [91; 32];
    let units: [[u8; 32]; 8] = std::array::from_fn(|index| [u8::try_from(index + 1).unwrap(); 32]);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(b"brutex.qualified-eight-slot-campaign.v1\0");
    h.update(&descriptor);
    for unit in units {
        h.update(&unit);
    }
    let identity = h.finalize();
    let directory = root
        .join("boolean-qualified-campaign-v1")
        .join(crate::server::hex32(identity));
    fs::create_dir_all(directory.join("0000000000000001")).unwrap();
    fs::write(directory.join("owner.lock"), []).unwrap();
    let mut body = b"BRQCAM01".to_vec();
    body.extend_from_slice(&identity);
    body.extend_from_slice(&descriptor);
    body.extend_from_slice(&[0; 40]);
    for unit in units {
        body.extend_from_slice(&unit);
        body.extend_from_slice(&[0; 1104]);
    }
    assert_eq!(body.len(), 9200);
    let mut bytes = b"BTXCHK01".to_vec();
    bytes.extend_from_slice(&identity);
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&9200_u64.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&body);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(&bytes);
    let pin = h.finalize();
    bytes.extend_from_slice(&pin);
    let path = directory.join("0000000000000001/payload");
    fs::write(&path, bytes).unwrap();
    fs::write(directory.join("0000000000000001/complete"), pin).unwrap();
    let value = render_qualified(
        &root,
        &Asked {
            identity,
            pin: Some(pin),
        },
    )
    .unwrap();
    assert_eq!(value["state"], "waiting");
    assert_eq!(value["history_records"], "1");
    assert_eq!(value["rows"].as_array().unwrap().len(), 8);
    assert_eq!(value["rows"][7]["rung"], "60min");
    assert_eq!(value["rows"][7]["unit"], crate::server::hex32([8; 32]));
    assert_eq!(value["child_completion_receipts_checked"], false);
    assert_eq!(value["child_bodies_checked"], false);
    assert!(value["rows"][0]["qualification"].is_null());
    assert!(
        render_qualified(
            &root,
            &Asked {
                identity,
                pin: Some([37; 32])
            }
        )
        .is_err()
    );
    fs::write(path, b"changed").unwrap();
    assert!(
        render_qualified(
            &root,
            &Asked {
                identity,
                pin: Some(pin)
            }
        )
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}
