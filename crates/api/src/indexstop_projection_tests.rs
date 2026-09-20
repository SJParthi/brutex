//! Independent saved-observation bytes prove the HTTP projection contract.
//! These generated records do not certify a market source or mint a producer.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "finite wire fixtures and exact response assertions"
)]

use super::*;
use brutex_core::blake3::{Hasher, hash};
use cli::index_stop_store::{ExitReason, Reason, ResearchFamilyV1};
use std::fs;

const ID: [u8; 32] = [0x61; 32];
const SOURCE: [u8; 32] = [0x62; 32];
const DAY: i64 = 20_000;
const MINUTE: i64 = 60_000_000;

struct Fixture {
    root: PathBuf,
    body: PathBuf,
    saved: Vec<u8>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.root);
    }
}

fn words(out: &mut Vec<u8>, values: &[u64]) {
    for word in values {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

fn observation_trade(direction: u8, signal: i64, entry: i64, stop: i64) -> Trade {
    Trade {
        signal_bar: 0,
        entry_bar: 1,
        exit_bar: 1,
        signal_micros: signal,
        signal_close_micros: entry,
        entry_micros: entry,
        exit_bar_micros: entry,
        exit_from_micros: entry,
        exit_until_micros: entry + MINUTE,
        stop_paisa: stop,
        entry_paisa: 10_000,
        optimistic_exit_paisa: stop,
        pessimistic_exit_paisa: if direction == 1 { 9_800 } else { 10_200 },
        optimistic_paisa: -100,
        pessimistic_paisa: -200,
        adverse_paisa: 200,
        favourable_paisa: 0,
        holding_minutes: 1,
        exit_reason: ExitReason::Stop,
        gapped: false,
    }
}

fn completion_receipt(bytes: &[u8]) -> Vec<u8> {
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&ID);
    receipt.extend_from_slice(&hash(bytes));
    words(&mut receipt, &[u64::try_from(bytes.len()).unwrap()]);
    receipt.extend_from_slice(&hash(&receipt));
    receipt
}

fn fixture() -> Fixture {
    let root = crate::scratch::path("single-stop-observation-pages");
    let directory = root.join(cli::index_stop_store::NAMESPACE).join(hex(ID));
    fs::create_dir_all(&directory).unwrap();
    let instrument = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let family = ResearchFamilyV1::new(instrument).unwrap();
    let program = vocab::expression::Expression::parse("0 | !1").unwrap();
    let mut bytes = b"BRISCT01".to_vec();
    bytes.extend_from_slice(&ID);
    bytes.extend_from_slice(&Policy::V1.canonical_bytes());
    words(&mut bytes, &[2]);
    for direction in [1_u8, 2] {
        let signal = DAY * 86_400_000_000 + 225 * MINUTE;
        let entry = signal + MINUTE;
        let stop = if direction == 1 { 9_900 } else { 10_100 };
        let trade = observation_trade(direction, signal, entry, stop);
        let event = Event {
            signal_bar: 0,
            signal_micros: signal,
            signal_close_micros: entry,
            stop_paisa: stop,
            reason: Reason::None,
            entry_bar: Some(1),
            entry_micros: Some(entry),
            occupied_through_micros: Some(entry),
            trade_index: Some(0),
        };
        let period = Period {
            day: DAY,
            offered_bars: 3,
            accepted_bars: 3,
            unavailable_minutes: 0,
            close_verified: false,
            trades: 1,
            wins: 0,
            optimistic_paisa: -100,
            pessimistic_paisa: -200,
            refused: 0,
            signals: 1,
        };
        let metrics = Metrics {
            trades: 1,
            optimistic_paisa: -100,
            pessimistic_paisa: -200,
            drawdown_paisa: 200,
            worst_trade_paisa: -200,
            adverse_paisa: 200,
            holding_minutes: 1,
            stopped: 1,
            ..Metrics::default()
        };
        let run_id = [direction; 32];
        let truth = [2_u64, 1, 1, 0];
        let children = [
            event.canonical_bytes().to_vec(),
            trade.canonical_bytes().to_vec(),
            period.canonical_bytes().to_vec(),
        ];
        // The fixture independently composes the documented observation seal.
        let mut seal = Hasher::new();
        seal.update(b"brutex.signal-candle-stop.evaluation.v1\0");
        seal.update(&SOURCE);
        seal.update(&run_id);
        seal.update(&metrics.canonical_bytes());
        for value in truth {
            seal.update(&value.to_le_bytes());
        }
        for child in &children {
            seal.update(&1_u64.to_le_bytes());
            seal.update(child);
        }
        for digest in [run_id, SOURCE, seal.finalize()] {
            bytes.extend_from_slice(&digest);
        }
        bytes.extend_from_slice(&family.encode());
        words(&mut bytes, &[u64::from(direction), 0]);
        bytes.extend_from_slice(&DAY.to_le_bytes());
        bytes.extend_from_slice(&DAY.to_le_bytes());
        bytes.extend_from_slice(&program.encode());
        words(&mut bytes, &truth);
        bytes.extend_from_slice(&metrics.canonical_bytes());
        words(&mut bytes, &[1, 1, 1]);
        for child in children {
            bytes.extend_from_slice(&child);
        }
    }
    let body = directory.join("body.bin");
    fs::write(directory.join("owner.lock"), []).unwrap();
    fs::write(&body, &bytes).unwrap();
    fs::write(directory.join("complete.bin"), completion_receipt(&bytes)).unwrap();
    Fixture {
        root,
        body,
        saved: bytes,
    }
}

#[test]
fn saved_stop_pages_keep_exact_links_pins_and_unassessed_policy() {
    let fixture = fixture();
    let base = format!("identity={}", hex(ID));
    let first = render(
        &fixture.root,
        &Asked::parse(&format!("{base}&limit=1")).unwrap(),
    )
    .unwrap();
    assert_eq!(first["status"], "saved");
    assert_eq!(first["total"], "2");
    assert_eq!(first["next"], "1");
    assert_eq!(first["rows"][0]["direction"], "long");
    assert_eq!(first["rows"][0]["metrics"]["pessimistic_paisa"], "-200");
    assert_eq!(first["rows"][0]["feed"], Value::Null);
    assert_eq!(first["policy"]["costs_included"], false);
    assert_eq!(
        first["policy"]["institutional_admission"],
        "not_assessed_in_this_view"
    );
    let pin = first["completion"].as_str().unwrap();
    let pinned = format!("{base}&completion={pin}");
    let second = render(
        &fixture.root,
        &Asked::parse(&format!("{pinned}&offset=1&limit=1")).unwrap(),
    )
    .unwrap();
    assert_eq!(second["rows"][0]["direction"], "short");
    assert_eq!(second["rows"][0]["program_index"], "0");
    assert_eq!(second["next"], Value::Null);
    for kind in ["trades", "events", "days"] {
        let asked = Asked::parse(&format!("{pinned}&kind={kind}&setting=1")).unwrap();
        let page = render(&fixture.root, &asked).unwrap();
        assert_eq!(page["total"], "1");
        assert_eq!(page["rows"][0]["index"], "0");
        assert_eq!(page["selected"]["direction"], "short");
        match kind {
            "trades" => {
                assert_eq!(page["rows"][0]["stop_paisa"], "10100");
                assert_eq!(page["rows"][0]["exit_reason"], "signal_candle_stop");
                assert_eq!(page["rows"][0]["pessimistic_paisa"], "-200");
            }
            "events" => {
                assert_eq!(page["rows"][0]["reason"], "priced");
                assert_eq!(page["rows"][0]["trade_index"], "0");
            }
            _ => {
                assert_eq!(page["rows"][0]["day"], DAY.to_string());
                assert_eq!(page["rows"][0]["close_verified"], false);
            }
        }
        let last = render(
            &fixture.root,
            &Asked::parse(&format!("{pinned}&kind={kind}&setting=1&offset=1")).unwrap(),
        )
        .unwrap();
        assert_eq!(last["rows"], json!([]));
        assert_eq!(last["next"], Value::Null);
    }
    for suffix in ["&kind=trades&setting=2", "&offset=3"] {
        assert!(
            render(
                &fixture.root,
                &Asked::parse(&format!("{pinned}{suffix}")).unwrap()
            )
            .is_err()
        );
    }
    let foreign = Asked::parse(&format!("{base}&completion={}", hex([0x71; 32]))).unwrap();
    assert!(
        render(&fixture.root, &foreign)
            .unwrap_err()
            .contains("completion changed")
    );
    let mut changed = fixture.saved.clone();
    changed[0] ^= 1;
    fs::write(&fixture.body, changed).unwrap();
    assert!(render(&fixture.root, &Asked::parse(&pinned).unwrap()).is_err());
    fs::write(&fixture.body, &fixture.saved).unwrap();
    let reopened = render(&fixture.root, &Asked::parse(&base).unwrap()).unwrap();
    assert_eq!(reopened["completion"], first["completion"]);
    assert_eq!(fs::read(&fixture.body).unwrap(), fixture.saved);
}

#[tokio::test]
async fn malformed_stop_requests_return_a_refusal_without_touching_the_store() {
    let uri: Uri = "/index-stop.json?identity=bad".parse().unwrap();
    let (status, headers, body) = index_stop_json(uri).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(headers[0].1, "application/json");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["status"], "refused");
    assert_eq!(body["rows"], json!([]));
    assert!(body["refusal"].as_str().is_some_and(|why| !why.is_empty()));
}
