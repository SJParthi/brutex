#![cfg(test)]
//! Generated saved companions exercise projection without any market store.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use brutex_core::blake3::{Hasher, hash};
use cli::index_stop_store::Reader as Catalog;
use std::fs;

const NAMESPACE: &str = "index-stop-vix-reference-v1";

fn words(bytes: &mut Vec<u8>, values: &[u64]) {
    for word in values {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
}

fn saved_reference(root: &Path, catalog: &Catalog, tag: u64) -> PathBuf {
    let mut lookup = Hasher::new();
    lookup.update(b"brutex-index-stop-vix-reference-lookup-v1\0");
    lookup.update(&catalog.identity());
    lookup.update(&catalog.completion_digest());
    let id = lookup.finalize();
    let records = catalog.records();
    assert_eq!(records.len(), 2);
    let first = &records[0].trades()[0];
    let month = pull::session::IstMoment::from_epoch_secs(first.entry_micros / 1_000_000)
        .expect("generated timestamp")
        .day()
        .year_month()
        .expect("generated month");
    let mut bytes = b"BRISVX01".to_vec();
    bytes.extend_from_slice(&id);
    bytes.extend_from_slice(&[0; 32]);
    bytes.extend_from_slice(&catalog.identity());
    bytes.extend_from_slice(&catalog.completion_digest());
    bytes.extend_from_slice(&hash(cli::index_stop_vix::POLICY.as_bytes()));
    let vendor = brutex_core::vendor::Vendor::ALL
        .iter()
        .position(|vendor| *vendor == brutex_core::vendor::Vendor::Zerodha)
        .expect("canonical generated feed");
    words(
        &mut bytes,
        &[u64::try_from(vendor).expect("vendor"), 2, 2, 1],
    );
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.trades().len(), 1);
        bytes.extend_from_slice(&record.run_id());
        bytes.extend_from_slice(&record.source_id());
        words(&mut bytes, &[u64::try_from(index).expect("setting"), 1]);
    }
    let unavailable = tag == 2;
    words(
        &mut bytes,
        &[
            u64::from(month.year()),
            u64::from(month.month()),
            u64::from(!unavailable),
            u64::from(tag == 1),
        ],
    );
    bytes.extend_from_slice(&if unavailable { [0; 32] } else { [0x73; 32] });
    let reason = if unavailable {
        "generated month unavailable"
    } else {
        ""
    };
    words(
        &mut bytes,
        &[u64::try_from(reason.len()).expect("reason length")],
    );
    bytes.extend_from_slice(reason.as_bytes());
    for record in records {
        let trade = &record.trades()[0];
        words(&mut bytes, &[0]);
        bytes.extend_from_slice(&record.run_id());
        bytes.extend_from_slice(&hash(&trade.canonical_bytes()));
        for micros in [
            trade.entry_micros,
            trade.exit_bar_micros,
            trade.exit_from_micros,
            trade.exit_until_micros,
        ] {
            bytes.extend_from_slice(&micros.to_le_bytes());
        }
        words(&mut bytes, &[0]);
        for micros in [trade.entry_micros, trade.exit_bar_micros] {
            words(&mut bytes, &[tag]);
            let fields = if tag == 1 {
                [micros, 1_500, 1_550, 1_490, 1_530, i64::MAX, i64::MIN]
            } else {
                [0; 7]
            };
            for value in fields {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    let mut publication = Hasher::new();
    publication.update(b"brutex-index-stop-vix-publication-v1\0");
    publication.update(&bytes[..40]);
    publication.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&publication.finalize());
    let directory = root.join(NAMESPACE).join(hex(id));
    fs::create_dir_all(&directory).expect("owned companion directory");
    fs::write(directory.join("owner.lock"), []).expect("owned publication lock");
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&id);
    receipt.extend_from_slice(&hash(&bytes));
    words(
        &mut receipt,
        &[u64::try_from(bytes.len()).expect("body length")],
    );
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(directory.join("complete.bin"), receipt).expect("sealed completion");
    let path = directory.join("body.bin");
    fs::write(&path, bytes).expect("generated reference body");
    path
}

#[test]
fn saved_vix_pages_authenticate_exact_absent_and_unavailable_companions_and_recover_cold() {
    for (tag, state, counter) in [
        (1, "exact", "exact_stamps"),
        (0, "absent", "absent_stamps"),
        (2, "unavailable", "unavailable_stamps"),
    ] {
        crate::indexstopjson::projection_tests::with_saved_stop_catalog(|root, id| {
            let catalog =
                Catalog::open(root, id, 1_048_576, 128).expect("generated native catalog");
            let pin = catalog.completion_digest();
            let body = saved_reference(root, &catalog, tag);
            let saved = fs::read(&body).expect("saved companion snapshot");
            let query = format!("identity={}&pin={}&setting=0", hex(id), hex(pin));
            let first = render(root, &Asked::parse(&query).expect("exact request"))
                .expect("authenticated companion");
            assert_eq!(first["status"], "saved");
            assert_eq!(first["reference_only"], true);
            assert_eq!(first["provenance_status"], "saved_reference_snapshot");
            assert_eq!(first["feed"], "zerodha");
            assert_eq!(first["catalog_identity"], hex(id));
            assert_eq!(first["catalog_completion"], hex(pin));
            assert_eq!(first["summary"][counter], "4");
            assert_eq!(first["summary"]["settings"], "2");
            assert_eq!(first["summary"]["trades"], "2");
            assert_eq!(first["rows"][0]["entry"]["state"], state);
            assert_eq!(first["rows"][0]["exit"]["state"], state);
            assert_eq!(
                first["rows"][0]["original_trade"]["pessimistic_paisa"],
                "-200"
            );
            assert_eq!(first["rows"][0]["trade_index"], "0");
            assert_eq!(first["total"], "1");
            assert!(first["next"].is_null());
            if tag == 1 {
                assert_eq!(
                    first["rows"][0]["entry"]["candle"]["open_interest"],
                    i64::MIN.to_string()
                );
                assert_eq!(
                    first["rows"][0]["exit"]["candle"]["volume"],
                    i64::MAX.to_string()
                );
            } else {
                assert!(first["rows"][0]["entry"]["candle"].is_null());
            }
            assert_eq!(
                first["rows"][0]["month"]["unavailable_code"].is_null(),
                tag != 2
            );
            if tag == 2 {
                assert_eq!(
                    first["rows"][0]["month"]["unavailable_reason"],
                    "generated month unavailable"
                );
            }
            let second = format!("identity={}&pin={}&setting=1", hex(id), hex(pin));
            let page = render(root, &Asked::parse(&second).expect("second setting"))
                .expect("same admitted catalog");
            assert_eq!(page["selected"]["direction"], "short");
            assert_eq!(page["reference"], first["reference"]);
            let end = render(
                root,
                &Asked::parse(&format!("{second}&offset=1")).expect("end page"),
            )
            .expect("exact empty end");
            assert_eq!(end["rows"], json!([]));
            assert_eq!(end["total"], "1");
            for request in [
                format!("{second}&offset=2"),
                format!("identity={}&pin={}&setting=2", hex(id), hex(pin)),
            ] {
                assert!(
                    render(
                        root,
                        &Asked::parse(&request).expect("bounded invalid coordinate")
                    )
                    .is_err()
                );
            }
            assert_eq!(
                render(root, &Asked::parse(&query).expect("original request"))
                    .expect("valid retry"),
                first
            );
            let mut corrupt = saved.clone();
            corrupt[0] ^= 1;
            fs::write(&body, corrupt).expect("corrupt only generated companion");
            assert!(render(root, &Asked::parse(&query).expect("same pin")).is_err());
            fs::write(&body, &saved).expect("restore exact companion");
            assert_eq!(
                render(root, &Asked::parse(&query).expect("cold retry"))
                    .expect("restored authority"),
                first
            );
            let foreign = format!("identity={}&pin={}&setting=0", hex(id), hex([0x79; 32]));
            assert!(render(root, &Asked::parse(&foreign).expect("foreign pin")).is_err());
            assert_eq!(fs::read(body).expect("projection preserves bytes"), saved);
        });
    }
}

#[tokio::test]
async fn malformed_reference_request_has_a_json_refusal_before_any_store_access() {
    let (status, headers, body) =
        index_stop_vix_json("/index-stop-vix.json?identity=bad".parse().expect("URI")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(headers[0].1, "application/json");
    let value: Value = serde_json::from_str(&body).expect("JSON refusal");
    assert_eq!(value["status"], "refused");
    assert_eq!(value["rows"], json!([]));
    assert!(value["refusal"].as_str().is_some_and(|why| !why.is_empty()));
}
