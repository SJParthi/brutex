//! Generated strict-source fixtures; never evidence of market profitability.
use super::*;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;

pub(crate) fn limits() -> Limits {
    Limits {
        programs: 4,
        records: 400_000,
        bytes: 128 * 1024 * 1024,
    }
}
pub(crate) fn source(fixture: &Fixture, symbol: &str, from: u8, to: u8) -> Result<Loaded, String> {
    fixture.prepare(
        symbol
            .strip_prefix("NSE-")
            .ok_or("generated canonical symbol")?,
    )?;
    let strict = StrictConfig::from_values(
        Some(fixture.root.as_os_str().to_owned()),
        Some("134217728".into()),
        Some("400000".into()),
    )
    .map_err(display)?;
    Loaded::load_identified(
        Request {
            store: &fixture.root,
            vendor: "zerodha",
            underlying: symbol,
            rung: "1min",
            from: (2025, from),
            to: (2025, to),
            strict: &strict,
        },
        "generated-index-stop-source-fixture",
    )
}
pub(crate) fn programs() -> Result<Vec<Expression>, String> {
    ["0 | !0", "0 & !0"]
        .iter()
        .map(|value| Expression::parse(value).map_err(|why| format!("{why:?}")))
        .collect()
}

pub(crate) fn load_generated(request: Request<'_>) -> Result<Loaded, String> {
    Loaded::load_identified(request, "generated-index-stop-source-fixture")
}

#[test]
fn strict_single_stop_saves_both_directions_and_both_fills_idempotently() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let programs = programs()?;
    let first = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    assert_eq!(first.evaluations().len(), 4);
    assert_eq!(
        first.evaluations().first().ok_or("long")?.direction(),
        Direction::Long
    );
    assert_eq!(
        first.evaluations().get(1).ok_or("short")?.direction(),
        Direction::Short
    );
    for evaluation in first.evaluations() {
        assert!(evaluation.truth().reconciles());
        assert_eq!(evaluation.policy(), Policy::V1);
        assert!(evaluation.metrics().optimistic_paisa >= evaluation.metrics().pessimistic_paisa);
        assert_eq!(
            evaluation.metrics().trades,
            u64::try_from(evaluation.trades().len()).map_err(display)?
        );
        for trade in evaluation.trades() {
            assert!(trade.optimistic_paisa >= trade.pessimistic_paisa);
            assert_eq!(
                runner::signal_candle_stop::Trade::decode(&trade.canonical_bytes())
                    .map_err(display)?,
                *trade
            );
        }
    }
    assert!(
        first.evaluations().first().ok_or("long")?.metrics().trades > 0,
        "known price condition should produce real fixture executions: {:?}",
        first.evaluations().first().ok_or("long")?.metrics()
    );
    assert_eq!(
        first
            .evaluations()
            .get(2)
            .ok_or("no-trade")?
            .metrics()
            .trades,
        0
    );
    let id = first.identity();
    let pin = first.completion_digest();
    drop(first);
    let second = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    assert_eq!((second.identity(), second.completion_digest()), (id, pin));
    second.require_current()?;
    Ok(())
}

#[test]
fn index_spot_vwap_unknown_is_not_made_true_by_or_not() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let program = Expression::parse("146 | !146").map_err(|why| format!("{why:?}"))?;
    let saved = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &[program],
        loaded.days(),
        limits(),
    )?;
    for reading in saved.evaluations() {
        assert_eq!(reading.metrics().trades, 0);
        assert_eq!(reading.truth().hits, 0);
        assert!(reading.truth().unknown > 0);
        assert!(reading.truth().reconciles());
    }
    Ok(())
}

#[test]
fn single_stop_retained_vix_publication_is_required_through_outer_terminal() -> Result<(), String> {
    for missing in [false, true] {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        let prepared = loaded.prepare(limits())?;
        let committed = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs()?,
            loaded.days(),
            limits(),
        )?;
        committed.require_current()?;
        let identity = committed.identity();
        let pin = committed.completion_digest();
        let lookup = crate::index_stop_vix::lookup_identity(identity, pin);
        let reference = fixture
            .output
            .join(crate::index_stop_vix::NAMESPACE)
            .join(crate::identity_hex(&lookup));
        let native = fixture
            .output
            .join(crate::index_stop_store::NAMESPACE)
            .join(crate::identity_hex(&identity));
        let before = std::fs::read(native.join("body.bin")).map_err(display)?;
        let owners = [
            crate::readonly_file::open(&native.join("owner.lock")).map_err(display)?,
            crate::readonly_file::open(&reference.join("owner.lock")).map_err(display)?,
        ];
        committed.with_publication_current(|| {
            for owner in &owners {
                assert!(owner.try_lock().is_err());
            }
            Ok(())
        })?;
        for owner in &owners {
            owner.try_lock().map_err(display)?;
            owner.unlock().map_err(display)?;
        }
        // Exercise the same final boundary used by the producer, after the
        // expensive decoded VIX/candidate reader has already been dropped.
        let outer = crate::sweep_evidence::begin(&fixture.output, identity, Operation::IndexStop)?;
        let receipt = reference.join("complete.bin");
        if missing {
            std::fs::remove_file(&receipt).map_err(display)?;
        } else {
            let mut bytes = std::fs::read(&receipt).map_err(display)?;
            *bytes.last_mut().ok_or("generated reference receipt")? ^= 1;
            std::fs::write(&receipt, bytes).map_err(display)?;
        }
        assert!(committed.require_current().is_err());
        assert!(finish_catalog(committed, outer).is_err());
        let latest = crate::sweep_evidence::read(&fixture.output, identity, limits().bytes)?
            .ok_or("outer terminal evidence")?;
        assert_eq!(latest.completion, Completion::Refused);
        assert_eq!(
            std::fs::read(native.join("body.bin")).map_err(display)?,
            before
        );
    }
    Ok(())
}

#[test]
fn single_stop_context_catalog_window_and_capture_admission_are_exact() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let foreign = source(&fixture, "NSE-BANKNIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let other = foreign.prepare(limits())?;
    let programs = programs()?;
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &other,
            &programs,
            loaded.days(),
            limits()
        )
        .is_err()
    );
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &[],
            loaded.days(),
            limits()
        )
        .is_err()
    );
    let duplicate = vec![programs.first().ok_or("program")?.clone(); 2];
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &duplicate,
            loaded.days(),
            limits()
        )
        .is_err()
    );
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            (loaded.days().0 - 1, loaded.days().1),
            limits()
        )
        .is_err()
    );
    let changed = Limits {
        records: limits().records - 1,
        ..limits()
    };
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            changed
        )
        .is_err()
    );
    assert!(validate_scope("NSE-RELIANCE", "1min").is_err());
    assert!(validate_scope("NSE-NIFTY", "day").is_err());
    assert!(
        validate_limits(Limits {
            programs: u64::MAX,
            ..limits()
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn later_single_stop_uses_original_first_history_and_a_bound_output_window() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let training = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let later = source(&fixture, "NSE-NIFTY", 5, 6)?;
    let window = (
        i64::from(
            pull::session::Day::new(2025, 6, 1)
                .map_err(display)?
                .days_from_epoch(),
        ),
        later.days().1,
    );
    let prepared = later.prepare(limits())?;
    let programs = programs()?;
    let result = produce_catalog(
        &fixture.output,
        &later,
        &prepared,
        &programs,
        window,
        limits(),
    )?;
    for evaluation in result.evaluations() {
        assert_eq!((evaluation.first_day(), evaluation.last_day()), window);
        assert!(
            evaluation
                .periods()
                .iter()
                .all(|day| day.day >= window.0 && day.day <= window.1)
        );
        assert!(evaluation.trades().iter().all(|trade| {
            (trade.entry_micros + indicators::IST_OFFSET_MICROS).div_euclid(86_400_000_000)
                >= window.0
        }));
    }
    assert!(training.days().1 < window.0);
    assert_eq!(later.days().0, training.days().0);
    assert_ne!(training.source_binding(), later.source_binding());
    Ok(())
}

#[test]
fn deleting_single_stop_receipt_refuses_warm_and_cold_observation() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let programs = programs()?;
    let result = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    let id = result.identity();
    let complete = fixture
        .output
        .join("index-stop-candidates-v1")
        .join(crate::identity_hex(&id))
        .join("complete.bin");
    std::fs::remove_file(complete).map_err(display)?;
    assert!(result.require_current().is_err());
    assert!(
        crate::index_stop_store::Reader::open(
            &fixture.output,
            id,
            limits().bytes,
            limits().records
        )
        .is_err()
    );
    Ok(())
}
