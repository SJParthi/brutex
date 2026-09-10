//! Independent sealed wire fixtures test observer projection, not market pricing.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded fixtures and assertions"
)]
use super::*;
use brutex_core::blake3::hash;
use std::fs;

const ID: [u8; 32] = [0x24; 32];
const ORIGINAL: [u8; 32] = [0x42; 32];
fn fixture(name: &str) -> PathBuf {
    let root = crate::booleanjson::tests::fixture(name);
    let parent = root
        .join("boolean-candidates-v1")
        .join(crate::server::hex32(ORIGINAL));
    let mut nested = fs::read(parent.join("body.bin")).unwrap();
    nested[8..40].copy_from_slice(&ID);
    let program = vocab::expression::Expression::parse("0 | !1")
        .unwrap()
        .encode();
    // The independent original fixture has one program, one session, two
    // 456-byte grid descriptors and two 512-byte coordinate/trade records.
    let session = 224 + program.len();
    nested[session..session + 8].copy_from_slice(&31_i64.to_le_bytes());
    let coordinates = session + 8 + 2 * 456;
    assert_eq!(nested.len(), coordinates + 2 * 512);
    let first = 2_691_900_000_000_i64; // 1970-02-01 09:15 IST, after January training.
    for offset in [coordinates, coordinates + 512] {
        nested[offset + 40..offset + 72].fill(9); // a distinct later execution run
        nested[offset + 392..offset + 400].copy_from_slice(&31_i64.to_le_bytes());
        nested[offset + 464..offset + 472].copy_from_slice(&(first + 60_000_000).to_le_bytes());
        nested[offset + 472..offset + 480].copy_from_slice(&(first + 120_000_000).to_le_bytes());
    }
    let mut body = b"BRBOOS01".to_vec();
    for digest in [
        ID,
        ORIGINAL,
        hash(&fs::read(parent.join("complete.bin")).unwrap()),
        [7; 32],
        [8; 32],
    ] {
        body.extend_from_slice(&digest);
    }
    body.extend_from_slice(&first.to_le_bytes());
    body.extend_from_slice(&(first + 540_000_000).to_le_bytes());
    for word in [10, 1970, 2, 1970, 2, nested.len() as u64] {
        body.extend_from_slice(&word.to_le_bytes());
    }
    assert_eq!(body.len(), 232);
    body.extend_from_slice(&nested);
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&ID);
    receipt.extend_from_slice(&hash(&body));
    receipt.extend_from_slice(&(body.len() as u64).to_le_bytes());
    receipt.extend_from_slice(&hash(&receipt));
    let later = root.join("boolean-oos-v1").join(crate::server::hex32(ID));
    fs::create_dir_all(&later).unwrap();
    fs::write(later.join("owner.lock"), []).unwrap();
    fs::write(later.join("body.bin"), body).unwrap();
    fs::write(later.join("complete.bin"), receipt).unwrap();
    root
}
#[test]
fn selectors_require_exact_pin_and_coordinate_without_unbounded_or_foreign_fields() {
    let id = crate::server::hex32(ID);
    assert!(Asked::parse(&format!("identity={id}")).is_ok());
    assert!(
        Asked::parse(&format!(
            "identity={id}&completion={id}&kind=trades&candidate=0&limit=256"
        ))
        .is_ok()
    );
    for suffix in [
        "&kind=trades&candidate=0",
        "&offset=1",
        "&offset=01",
        "&candidate=0",
        "&kind=grid",
        "&path=/tmp",
        "&limit=257",
        "&limit=0",
        "&completion=",
        "&identity=x",
        "&candidate=0&candidate=0",
    ] {
        assert!(
            Asked::parse(&format!("identity={id}{suffix}")).is_err(),
            "{suffix}"
        );
    }
}
#[test]
fn sealed_later_projection_preserves_original_link_exact_settings_own_trades_and_zero_safe_sessions()
 {
    let root = fixture("boolean-later-projection");
    let reader = LaterPeriod::open(&root, ID, crate::detail::MAX_SCAN_BYTES).unwrap();
    let pin = reader.completion_digest();
    let asked = Asked::parse(&format!("identity={}", crate::server::hex32(ID))).unwrap();
    let body = project(&reader, &asked).unwrap();
    assert_eq!(body["parent"]["identity"], crate::server::hex32(ORIGINAL));
    assert_eq!(body["rows"].as_array().unwrap().len(), 2);
    assert_eq!(
        body["rows"][0]["training"]["cell"],
        body["rows"][0]["later"]["cell"]
    );
    assert_ne!(
        body["rows"][0]["training"]["run"],
        body["rows"][0]["later"]["run"]
    );
    assert_eq!(body["later"]["bars"], "10");
    assert_eq!(body["later"]["first_micros"], "2691900000000");
    for kind in ["trades", "sessions"] {
        let asked = Asked::parse(&format!(
            "identity={}&completion={}&kind={kind}&candidate=1",
            crate::server::hex32(ID),
            crate::server::hex32(pin)
        ))
        .unwrap();
        let page = project(&reader, &asked).unwrap();
        assert_eq!(page["total"], "1");
        assert!(page["next"].is_null());
        assert_eq!(page["selected"]["index"], "1");
        if kind == "trades" {
            assert_eq!(page["rows"][0]["entry_micros"], "2691960000000");
            assert_eq!(page["rows"][0]["worst"], "10");
        } else {
            assert_eq!(page["rows"][0]["day"], "31");
            assert_eq!(page["rows"][0]["trades"], "1");
        }
    }
    let wrong = Asked {
        completion: Some([1; 32]),
        identity: asked.identity,
        kind: asked.kind.clone(),
        candidate: asked.candidate,
        offset: asked.offset,
        limit: asked.limit,
    };
    assert!(project(&reader, &wrong).is_err());
    fs::write(
        root.join("boolean-candidates-v1")
            .join(crate::server::hex32(ORIGINAL))
            .join("complete.bin"),
        [],
    )
    .unwrap();
    assert!(project(&reader, &asked).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lowered_budget_refuses_cached_later_ancestry_and_original_pin_recovers_after_readmission() {
    let root = fixture("boolean-later-budget");
    let admitted = crate::detail::BooleanObservationBudget::from_value(None).unwrap();
    let tiny = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new("1")))
        .unwrap();
    let initial = render_with_budget(
        &root,
        &Asked::parse(&format!("identity={}", crate::server::hex32(ID))).unwrap(),
        admitted,
    )
    .unwrap();
    let asked = Asked::parse(&format!(
        "identity={}&completion={}",
        crate::server::hex32(ID),
        initial["completion"].as_str().unwrap()
    ))
    .unwrap();
    let why = render_with_budget(&root, &asked, tiny).unwrap_err();
    assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
    let reopened = render_with_budget(&root, &asked, admitted).unwrap();
    assert_eq!(reopened["completion"], initial["completion"]);
    assert_eq!(reopened["rows"], initial["rows"]);
    assert_eq!(reopened["observation_byte_limit"], "67108864");
    fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn public_later_route_refuses_invalid_selectors_without_accessing_a_store() {
    let reply = later_json("/boolean-oos.json?identity=bad".parse().unwrap()).await;
    assert_eq!(reply.0, StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_str(&reply.2).unwrap();
    assert_eq!(body["rows"], json!([]));
}
