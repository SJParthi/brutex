//! Generated OHLCV tests validate observations only, never market performance.
#![expect(clippy::unwrap_used, reason = "named fixture assertions")]
use super::*;
use crate::candidate_universe::boolean_candidate_v1::persistence;
use brutex_core::blake3::hash;
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use runner::identity::{DailyReferenceBinding, Params, ReferenceIntegrity};
use runner::signal_candle_stop::{Prepared, Source};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
const ID: [u8; 32] = [11; 32];
const BUDGET: u64 = 8_000_000;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "brutex-index-stop-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn evaluations() -> Vec<Evaluation> {
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().unwrap(),
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    let instrument = InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap();
    let source = Source {
        series: runner::exit_grid_policy::ExecutionSeriesV1::new(
            &instrument,
            "zerodha",
            "generated-test-build",
            [7; 32],
            &bars,
        )
        .unwrap(),
        signal_bars: &bars,
        signal_column: &column,
        minute_context: &bars,
        daily_reference: DailyReferenceBinding {
            daily_bars: &[],
            eligibility: &[],
            schema: 1,
            eligibility_policy: 1,
            gap_overlay_policy: 1,
            excluded_ist_days: &[],
            daily_integrity: ReferenceIntegrity::ChecksumReceiptV1([8; 32]),
            minute_integrity: ReferenceIntegrity::ChecksumReceiptV1([9; 32]),
            swept_series_calendar_policy: 1,
        },
        timeframe: "1min",
        first_day: 0,
        last_day: 7,
        params: Params {
            min_hits: 0,
            ceiling: 10000,
            pair_budget: 2,
            policy: 1,
        },
    };
    let prepared = Prepared::new(Policy::V1, source).unwrap();
    let program = Expression::parse("0 | !0").unwrap();
    [Direction::Long, Direction::Short]
        .into_iter()
        .map(|side| prepared.evaluate(&program, side, 10000).unwrap())
        .collect()
}
fn publish(root: &Path, body: &[u8]) {
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, ID, body).unwrap();
    pending.verify_body(hash(body), body.len() as u64).unwrap();
    pending.finish(ID, hash(body), body.len() as u64).unwrap();
}
#[test]
fn native_roundtrip_retains_complete_program_both_sides_fills_and_all_events() {
    let evaluations = evaluations();
    let body = encode(ID, &evaluations, BUDGET).unwrap();
    let rows = decode(&body, ID, 100_000).unwrap();
    assert_eq!(rows.len(), 2);
    for (row, native) in rows.iter().zip(&evaluations) {
        assert_eq!(row.run_id(), native.run_id());
        assert_eq!(row.source_id(), native.source_id());
        assert_eq!(row.evaluation_digest(), native.digest());
        assert_eq!(row.program(), native.program());
        assert_eq!(row.truth(), native.truth());
        assert_eq!(row.metrics(), native.metrics());
        assert_eq!(row.trades(), native.trades());
        assert_eq!(row.events(), native.events());
        assert_eq!(row.periods(), native.periods());
        assert_eq!(row.timeframe(), "1min");
    }
    assert_eq!(body, encode(ID, &evaluations, BUDGET).unwrap());
    assert!(rows.iter().any(|r| !r.trades().is_empty()));
}
#[test]
fn publisher_and_cold_reader_refuse_partial_pairs_counts_truncation_and_mutated_rows() {
    let evaluations = evaluations();
    let body = encode(ID, &evaluations, BUDGET).unwrap();
    assert!(encode(ID, &[], BUDGET).is_err());
    assert!(encode(ID, evaluations.get(..1).unwrap(), BUDGET).is_err());
    assert!(encode([0; 32], &evaluations, BUDGET).is_err());
    assert!(encode(ID, &evaluations, body.len() as u64 + 111).is_err());
    assert!(decode(&body, ID, 1).is_err());
    assert!(decode(&body, [12; 32], 100_000).is_err());
    for cut in [0, 7, 39, 127, 128, body.len() - 1] {
        assert!(
            decode(body.get(..cut).unwrap(), ID, 100_000).is_err(),
            "cut {cut}"
        );
    }
    for at in [
        0,
        8,
        40,
        120,
        128,
        160,
        192,
        224,
        352,
        360,
        368,
        376,
        384,
        body.len() - 1,
    ] {
        let mut bad = body.clone();
        *bad.get_mut(at).unwrap() ^= 1;
        assert!(decode(&bad, ID, 100_000).is_err(), "byte {at}");
    }
    let mut bad = body.clone();
    bad.push(0);
    assert!(decode(&bad, ID, 100_000).is_err());
}
#[test]
fn cold_read_and_warm_pins_refuse_changed_body_receipt_or_owner_without_substitution() {
    let dir = Temp::new();
    let body = encode(ID, &evaluations(), BUDGET).unwrap();
    publish(&dir.0, &body);
    let reader = Reader::open(&dir.0, ID, BUDGET, 100_000).unwrap();
    let pin = reader.completion_digest();
    assert_eq!(reader.identity(), ID);
    assert_eq!(reader.admitted_bytes(), body.len() as u64 + 112);
    assert!(reader.record(pin, 0).is_ok());
    assert!(reader.record([22; 32], 0).is_err());
    assert!(reader.record(pin, 2).is_err());
    let owner = std::fs::File::open(
        dir.0
            .join(NAMESPACE)
            .join(crate::identity_hex(&ID))
            .join("owner.lock"),
    )
    .unwrap();
    reader
        .with_current(|| {
            assert!(owner.try_lock().is_err());
            assert_eq!(reader.records().len(), 2);
            assert!(owner.try_lock().is_err());
            Ok(())
        })
        .unwrap();
    owner.try_lock().unwrap();
    owner.unlock().unwrap();
    assert!(Reader::open(&dir.0, ID, body.len() as u64 + 111, 100_000).is_err());
    let path = dir
        .0
        .join(NAMESPACE)
        .join(crate::identity_hex(&ID))
        .join("body.bin");
    let mut changed = body;
    *changed.last_mut().unwrap() ^= 1;
    std::fs::write(path, changed).unwrap();
    assert!(reader.require_current().is_err());
    assert!(reader.record(pin, 0).is_err());
    assert!(Reader::open(&dir.0, ID, BUDGET, 100_000).is_err());
}
#[test]
fn immutable_receipt_owner_and_byte_identical_replacement_revoke_old_pins() {
    let body = encode(ID, &evaluations(), BUDGET).unwrap();
    for file in ["body.bin", "complete.bin", "owner.lock"] {
        let dir = Temp::new();
        publish(&dir.0, &body);
        let reader = Reader::open(&dir.0, ID, BUDGET, 100_000).unwrap();
        let pin = reader.completion_digest();
        let parent = dir.0.join(NAMESPACE).join(crate::identity_hex(&ID));
        let original = parent.join(file);
        let bytes = std::fs::read(&original).unwrap();
        let replacement = parent.join("replacement");
        std::fs::write(&replacement, bytes).unwrap();
        std::fs::rename(replacement, &original).unwrap();
        let mut called = false;
        assert!(
            reader
                .with_current(|| {
                    called = true;
                    Ok(())
                })
                .is_err(),
            "{file}"
        );
        assert!(!called, "stale {file} cannot publish even unchanged bytes");
        assert!(reader.record(pin, 0).is_err());
        // A genuinely new cold admission may authenticate identical replacement
        // bytes. It must acquire the new generation instead of keeping this view.
        let fresh = Reader::open(&dir.0, ID, BUDGET, 100_000).unwrap();
        fresh.require_current().unwrap();
        std::fs::remove_file(&original).unwrap();
        assert!(fresh.require_current().is_err());
        assert!(Reader::open(&dir.0, ID, BUDGET, 100_000).is_err());
    }
    for cut in [0, 111] {
        let dir = Temp::new();
        publish(&dir.0, &body);
        let path = dir
            .0
            .join(NAMESPACE)
            .join(crate::identity_hex(&ID))
            .join("complete.bin");
        let receipt = std::fs::read(&path).unwrap();
        std::fs::write(path, receipt.get(..cut).unwrap()).unwrap();
        assert!(Reader::open(&dir.0, ID, BUDGET, 100_000).is_err());
    }
}
#[test]
fn reconciliation_rejects_changed_direction_duplicate_trade_links_daily_totals_and_observation_digest()
 {
    let body = encode(ID, &evaluations(), BUDGET).unwrap();
    let rows = decode(&body, ID, 100_000).unwrap();
    let row = rows.first().unwrap();
    for change in [0, 1, 2, 3, 4] {
        let mut bad = row.clone();
        match change {
            0 => bad.metrics.wins = bad.metrics.wins.saturating_add(1),
            1 => bad.evaluation_digest = [9; 32],
            2 => bad.direction = Direction::Short,
            3 => bad.periods.first_mut().unwrap().pessimistic_paisa += 1,
            _ => {
                bad.events
                    .iter_mut()
                    .find(|event| event.trade_index.is_some())
                    .unwrap()
                    .trade_index = Some(99_999);
            }
        }
        assert!(validate(&bad).is_err());
    }
    let mut bad = row.clone();
    bad.program = Expression::parse("1").unwrap();
    assert!(validate_pair(row, &bad).is_err());
}
#[test]
fn reconciliation_checks_underlying_rows_even_after_the_observation_digest_is_updated() {
    let body = encode(ID, &evaluations(), BUDGET).unwrap();
    let rows = decode(&body, ID, 100_000).unwrap();
    let row = rows.first().unwrap();
    let priced = row
        .events
        .iter()
        .position(|e| e.trade_index.is_some())
        .unwrap();
    for change in 0..9 {
        let mut bad = row.clone();
        match change {
            0 => bad.metrics.pessimistic_paisa += 1,
            1 => bad.periods.first_mut().unwrap().signals += 1,
            2 => bad.events.get_mut(priced).unwrap().trade_index = Some(1),
            3 => bad.events.get_mut(priced).unwrap().stop_paisa += 1,
            4 => bad.events.get_mut(priced).unwrap().signal_close_micros += 1,
            5 => bad.events.get_mut(priced).unwrap().reason = Reason::WhileOpen,
            6 => {
                bad.trades.get_mut(1).unwrap().entry_micros =
                    bad.trades.first().unwrap().entry_micros;
            }
            7 => bad.periods.get_mut(1).unwrap().day = bad.periods.first().unwrap().day,
            _ => bad.periods.first_mut().unwrap().pessimistic_paisa += 1,
        }
        bad.evaluation_digest = observation_digest(
            bad.source_id,
            bad.run_id,
            bad.truth,
            bad.metrics,
            &bad.events,
            &bad.trades,
            &bad.periods,
        );
        assert!(validate(&bad).is_err(), "consistency mutation {change}");
    }
    let mut bad = rows.clone();
    bad.reverse();
    assert!(validate_pair(bad.first().unwrap(), bad.get(1).unwrap()).is_ok());
    // Pair content alone is not ordering authority: the body decoder checks the
    // fixed long/short positions separately before admitting a candidate page.
    let mut encoded = body.clone();
    *encoded.get_mut(HEADER_BYTES + 96 + 128).unwrap() = 2;
    assert!(decode(&encoded, ID, 100_000).is_err());
}
