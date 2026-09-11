//! Finite wire fixtures test observation only; these bytes are not market proof.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded independent wire fixtures and assertions"
)]
use super::*;
use brutex_core::blake3::hash;
use std::fs;

const ID: &str = "4242424242424242424242424242424242424242424242424242424242424242";
static CACHE_TEST: Mutex<()> = Mutex::new(());
fn words(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}
pub(crate) fn fixture(name: &str) -> PathBuf {
    let root = crate::scratch::path(name);
    fs::create_dir_all(&root).unwrap();
    let directory = root.join("boolean-candidates-v1").join(ID);
    fs::create_dir_all(&directory).unwrap();
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let family = cli::boolean_observation::ResearchFamilyV1::new(key).unwrap();
    let program = vocab::expression::Expression::parse("0 | !1").unwrap();
    let mut body = b"BRBOOL01".to_vec();
    body.extend_from_slice(&[0x42; 32]);
    body.extend_from_slice(&[0x19; 32]);
    body.extend_from_slice(&family.encode());
    words(&mut body, &[1, 1, 2]);
    body.extend_from_slice(&program.encode());
    words(&mut body, &[0]);
    for side in 0..2 {
        body.extend_from_slice(&[1; 160]);
        words(&mut body, &[10, 1, 60_000_001, 1, 5, 60, 0, side]);
        for _ in 0..3 {
            words(&mut body, &[1, 1, 2]);
        }
        words(&mut body, &[1, 100, 10000, 100, 100, 0]);
        body.extend_from_slice(&[7; 32]);
        words(&mut body, &[0, 0, 100, 100]);
        for _ in 0..3 {
            words(&mut body, &[1, 1000]);
        }
    }
    for side in 0..2 {
        body.extend_from_slice(&[u8::try_from(side + 1).unwrap(); 32]);
        words(&mut body, &[0]);
        body.extend_from_slice(&[3; 32]);
        words(&mut body, &[side, 0]);
        // One 30-word measured cell: same exact trade on both sides for this
        // wire-contract fixture; it does not claim a priced market producer.
        let mut cell = vec![0_u64; 30];
        cell[..5].fill(u64::MAX);
        cell[5] = 1;
        cell[6] = 1;
        cell[7] = 10;
        cell[8] = 12;
        cell[14] = 1;
        words(&mut body, &cell);
        words(&mut body, &[3, 1, 1, 1, 10, 1, 4, 5]); // refusal, periods, trades, support, four truth counts
        words(&mut body, &[0, 10, 1, 1]);
        words(&mut body, &[0, 1, 2, 12, 10, 1, 60_000_001, 0, 0, 2, 2]);
    }
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&[0x42; 32]);
    receipt.extend_from_slice(&hash(&body));
    words(&mut receipt, &[body.len() as u64]);
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(directory.join("owner.lock"), []).unwrap();
    fs::write(directory.join("body.bin"), body).unwrap();
    fs::write(directory.join("complete.bin"), receipt).unwrap();
    root
}

#[test]
fn canonical_selectors_and_pins_refuse_cross_page_fallback() {
    assert!(Asked::parse(&format!("identity={ID}&limit=256")).is_ok());
    for suffix in [
        "&offset=1",
        "&offset=01",
        "&limit=0",
        "&limit=257",
        "&candidate=0",
        "&kind=trades",
        "&kind=grid&side=long&axis=stop",
        "&side=long",
        "&unknown=1",
        "&identity=bad",
        "&completion=bad",
        "&offset=18446744073709551616",
    ] {
        assert!(
            Asked::parse(&format!("identity={ID}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    assert!(
        Asked::parse(&format!(
            "identity={ID}&completion={ID}&kind=sessions&candidate=9007199254740993"
        ))
        .is_ok()
    );
    assert!(Asked::parse(&"x".repeat(513)).is_err());
}

#[test]
fn authenticated_catalog_projects_exact_program_coordinate_trade_session_and_grid_pages() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-projections");
    let first = render(
        &root,
        &Asked::parse(&format!("identity={ID}&limit=1")).unwrap(),
    )
    .unwrap();
    assert_eq!(first["rows"][0]["expression"], "(0 | !(1))");
    assert_eq!(first["next"], "1");
    assert_eq!(first["coordinate_count"], "2");
    assert_eq!(
        first["rows"][0]["execution_refusals"],
        json!(["Missing stop", "Missing target"])
    );
    let pin = first["completion"].as_str().unwrap();
    for (suffix, total) in [
        ("kind=programs", 1),
        ("kind=trades&candidate=0", 1),
        ("kind=sessions&candidate=0", 1),
        ("kind=grid&side=short&axis=stop", 1),
        ("kind=grid&side=long&axis=requested-target", 1),
        ("offset=1&limit=1", 2),
    ] {
        let page = render(
            &root,
            &Asked::parse(&format!("identity={ID}&completion={pin}&{suffix}")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["total"], total.to_string());
        assert_eq!(page["rows"].as_array().unwrap().len(), 1);
        assert_eq!(page["next"], Value::Null);
        if suffix.starts_with("kind=trades") {
            assert_eq!(page["rows"][0]["worst"], "10");
            assert_eq!(page["selected"]["index"], "0");
        }
        if suffix.starts_with("kind=sessions") {
            assert_eq!(page["rows"][0]["return_paisa"], "10");
        }
        if suffix.contains("axis=requested") {
            assert_eq!(page["rows"][0]["denominator"], "2");
        }
    }
    assert!(
        render(
            &root,
            &Asked::parse(&format!("identity={ID}&completion={ID}&offset=1")).unwrap()
        )
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_smaller_server_budget_cannot_reuse_a_previously_admitted_catalog() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-budget-cache");
    let admitted = crate::detail::BooleanObservationBudget::from_value(None).unwrap();
    let tiny = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new("1")))
        .unwrap();
    let first = render_with_budget(
        &root,
        &Asked::parse(&format!("identity={ID}")).unwrap(),
        admitted,
    )
    .unwrap();
    assert_eq!(first["observation_byte_limit"], "67108864");
    let asked = Asked::parse(&format!(
        "identity={ID}&completion={}",
        first["completion"].as_str().unwrap()
    ))
    .unwrap();
    let why = render_with_budget(&root, &asked, tiny).unwrap_err();
    assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
    let reopened = render_with_budget(&root, &asked, admitted).unwrap();
    assert_eq!(reopened["completion"], first["completion"]);
    assert_eq!(reopened["rows"], first["rows"]);
    fs::write(
        root.join("boolean-candidates-v1")
            .join(ID)
            .join("complete.bin"),
        [],
    )
    .unwrap();
    assert!(
        render_with_budget(&root, &asked, admitted).is_err(),
        "a sufficient budget does not forgive receipt corruption"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_busy_replaced_or_truncated_evidence_never_returns_saved_empty() {
    let _cache = CACHE_TEST.lock().unwrap();
    let missing = crate::scratch::path("boolean-api-missing");
    assert!(render(&missing, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    let root = fixture("boolean-api-refusals");
    let reader = Reader::open(&root, [0x42; 32], crate::detail::MAX_SCAN_BYTES).unwrap();
    let directory = root.join("boolean-candidates-v1").join(ID);
    let owner = std::fs::File::open(directory.join("owner.lock")).unwrap();
    owner.try_lock().unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    owner.unlock().unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_ok());
    fs::write(directory.join("body.bin"), b"changed").unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_integer_projection_and_page_extents_are_not_rounded_or_truncated() {
    let row = TradeRow {
        signal_bar: 9_007_199_254_740_993,
        entry_bar: 1,
        exit_bar: 2,
        best: i64::MAX,
        worst: i64::MIN,
        entry_micros: 1,
        exit_micros: 2,
        adverse: 0,
        adverse_paisa: 0,
        favourable: 1,
        favourable_paisa: 1,
    };
    let value = trade(0, row);
    assert_eq!(value["signal_bar"], "9007199254740993");
    assert_eq!(value["worst"], i64::MIN.to_string());
    assert_eq!(next_offset(0, 0, 32, 0).unwrap(), None);
    assert_eq!(next_offset(257, 0, 256, 256).unwrap(), Some(256));
    assert!(next_offset(257, 0, 256, 255).is_err());
    assert!(next_offset(257, 258, 1, 0).is_err());
    assert!(refusal_names(64).is_err());
}

#[test]
fn pinned_cache_refuses_replacement_while_an_explicit_new_read_authenticates_again() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-cache-refresh");
    let first = Asked::parse(&format!("identity={ID}")).unwrap();
    let body = render(&root, &first).unwrap();
    let pin = body["completion"].as_str().unwrap();
    let pinned = Asked::parse(&format!("identity={ID}&completion={pin}")).unwrap();
    let path = root.join("boolean-candidates-v1").join(ID).join("body.bin");
    let replacement = path.with_extension("replacement");
    fs::copy(&path, &replacement).unwrap();
    fs::rename(replacement, &path).unwrap();
    assert!(render(&root, &pinned).is_err());
    assert_eq!(render(&root, &first).unwrap()["completion"], pin);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn public_handler_refuses_bad_selectors_before_reading_any_root() {
    let (status, _, body) =
        boolean_json(Uri::from_static("/boolean-candidates.json?identity=bad")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["status"],
        "refused"
    );
}
