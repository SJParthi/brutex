//! Generated numeric fixtures only; no market-source capability is fabricated.
use super::*;
use crate::population_statistics_v2::{cscv_placement, wilson_lower_bits};
use runner::bootstrap::{
    romano_wolf_adjusted_p_values_v1, spa_receipt_v1, white_reality_check_receipt_v1,
};

fn procedure() -> Result<PopulationStatisticsProcedureV2, String> {
    PopulationStatisticsProcedureV2::new(31, 72, 2)
}
fn limits() -> Bounds {
    Bounds {
        families: 2,
        candidates: 32,
        observations: 256,
        bootstrap_work: 100_000,
        split_work: 100_000,
        memory_bytes: 2_000_000,
        bytes: 100_000,
    }
}
fn candidates(returns: &[Vec<i64>]) -> Vec<CandidateStatisticsV1> {
    returns
        .iter()
        .enumerate()
        .map(|(index, values)| CandidateStatisticsV1 {
            family: index / 2,
            coordinate: index % 2,
            identity: [u8::try_from(index).unwrap_or(255); 32],
            trades: values.len() as u64,
            wins: values.iter().filter(|value| **value > 0).count() as u64,
            return_paisa: values.iter().sum(),
            wilson_lower_bits: wilson_lower_bits(
                values.iter().filter(|value| **value > 0).count() as u64,
                values.len() as u64,
            ),
            period_digest: [u8::try_from(index + 1).unwrap_or(255); 32],
        })
        .collect()
}

#[test]
fn complete_program_population_matches_existing_exact_bootstrap_receipts() -> Result<(), String> {
    let returns = vec![
        vec![10, -3, 5, 7],
        vec![-9, 6, -2, 3],
        vec![2, 8, -4, -1],
        vec![-3, 4, 1, -6],
    ];
    let procedure = procedure()?;
    let layout = derive_layout(4)?;
    let measured = numeric::measure_returns(&returns, candidates(&returns), layout, procedure)?;
    assert_eq!(
        measured.white,
        white_reality_check_receipt_v1(&returns, 31, 72, 2).ok_or("White")?
    );
    assert_eq!(
        measured.spa,
        spa_receipt_v1(&returns, 31, 72, 2).ok_or("SPA")?
    );
    assert_eq!(
        measured.romano,
        romano_wolf_adjusted_p_values_v1(&returns, 31, 72, 2)
    );
    assert_eq!(
        measured.romano_availability,
        RomanoWolfAvailabilityV1::Measured
    );
    assert_eq!(measured.candidates.len(), 4);
    assert_eq!(measured.white.strategies(), 4);
    assert_eq!(measured.white.exact_p_value().denominator(), 32);
    // Independent explicit complements for four single-session blocks: bit0
    // belongs to test, every remaining two-bit training subset occurs once.
    let masks = [(6, 9), (10, 5), (12, 3)];
    assert_eq!(measured.splits.len(), masks.len());
    let mut bottom = 0;
    for (split, (train_mask, test_mask)) in measured.splits.iter().zip(masks) {
        assert_eq!((split.train_mask, split.test_mask), (train_mask, test_mask));
        let scores = |mask| {
            returns
                .iter()
                .map(|values| {
                    values
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| mask & (1 << index) != 0)
                        .map(|(_, value)| *value)
                        .sum()
                })
                .collect::<Vec<i64>>()
        };
        let expected = cscv_placement(&scores(train_mask), &scores(test_mask))?;
        assert_eq!((split.bottom_half, split.rankable), expected);
        bottom += u64::from(expected.0);
        assert_ne!(split.scores_digest, [0; 32]);
    }
    assert_eq!(
        (measured.contributing_splits, measured.bottom_half_splits),
        (3, bottom)
    );
    Ok(())
}

#[test]
fn zero_trade_coordinate_remains_in_family_and_cannot_gain_rw_evidence() -> Result<(), String> {
    let mut returns = vec![vec![10, -3, 5, 7], vec![-9, 6, -2, 3]];
    let without = numeric::measure_returns(
        &returns,
        candidates(&returns),
        derive_layout(4)?,
        procedure()?,
    )?;
    assert!(without.romano.is_some());
    returns.push(vec![0; 4]);
    let mut rows = candidates(&returns);
    let zero = rows.last_mut().ok_or("zero coordinate")?;
    zero.trades = 0;
    zero.wins = 0;
    zero.wilson_lower_bits = wilson_lower_bits(0, 0);
    let measured = numeric::measure_returns(&returns, rows, derive_layout(4)?, procedure()?)?;
    assert_eq!(measured.candidates.len(), 3);
    assert_eq!(measured.white.strategies(), 3);
    assert_eq!(measured.spa.strategies(), 3);
    assert_eq!(measured.romano, None);
    assert_eq!(
        measured.romano_availability,
        RomanoWolfAvailabilityV1::ConstantReturnCandidate
    );
    assert!(
        measured
            .romano_availability
            .label()
            .contains("constant-return")
    );
    assert_eq!(measured.candidates.last().ok_or("last")?.trades, 0);
    assert_ne!(
        without.white.family_digest(),
        measured.white.family_digest()
    );
    Ok(())
}

#[test]
fn explicit_physical_limits_and_overflow_refuse_before_shared_numeric_work() -> Result<(), String> {
    let base = limits();
    admit_shape(4, 4, 3, procedure()?, base, 5632)?;
    for bounds in [
        Bounds {
            candidates: 3,
            ..base
        },
        Bounds {
            observations: 15,
            ..base
        },
        Bounds {
            bootstrap_work: 1487,
            ..base
        },
        Bounds {
            split_work: 47,
            ..base
        },
        Bounds {
            memory_bytes: 1,
            ..base
        },
        Bounds { bytes: 0, ..base },
    ] {
        assert!(admit_shape(4, 4, 3, procedure()?, bounds, 5632).is_err());
    }
    assert!(admit_shape(u64::MAX, 2, 1, procedure()?, base, 5632).is_err());
    assert!(
        admit_shape(
            4,
            4,
            3,
            PopulationStatisticsProcedureV2::new(u64::MAX, 0, 1)?,
            base,
            5632
        )
        .is_err()
    );
    assert!(admit_shape(0, 4, 3, procedure()?, base, 5632).is_err());
    Ok(())
}

#[test]
fn common_memory_and_output_cap_charges_actual_body_not_the_unused_ceiling() -> Result<(), String> {
    let bounds = Bounds {
        memory_bytes: 100_000,
        bytes: 100_000,
        ..limits()
    };
    admit_shape(4, 4, 3, procedure()?, bounds, 5632)?;
    assert!(admit_shape(4, 4, 3, procedure()?, bounds, 100_001).is_err());
    assert!(admit_shape(4, 4, 3, procedure()?, bounds, u64::MAX).is_err());
    Ok(())
}

#[test]
fn cscv_keeps_all_periods_and_refuses_misalignment_without_padding() -> Result<(), String> {
    assert!(derive_layout(3).is_err());
    let layout = derive_layout(6)?;
    assert_eq!(layout.period_count(), 6);
    let returns = vec![vec![1, 2, 3, 4, 5, 6], vec![6, 5, 4, 3, 2, 1]];
    let measured = numeric::measure_returns(&returns, candidates(&returns), layout, procedure()?)?;
    assert_eq!(measured.white.periods(), 6);
    assert_eq!(measured.splits.len() as u64, layout.split_count());
    let mut missing = returns.clone();
    missing.get_mut(1).ok_or("second")?.pop();
    assert!(
        numeric::measure_returns(&missing, candidates(&missing), layout, procedure()?).is_err()
    );
    assert!(numeric::measure_returns(&returns, Vec::new(), layout, procedure()?).is_err());
    Ok(())
}

pub(super) fn stored_limits() -> Bounds {
    Bounds {
        families: 2,
        candidates: 1024,
        observations: 50_000,
        bootstrap_work: 10_000_000,
        split_work: 10_000_000,
        memory_bytes: 64 * 1024 * 1024,
        bytes: 4 * 1024 * 1024,
    }
}

#[test]
fn actual_opaque_cash_and_index_sources_publish_complete_idempotent_statistics()
-> Result<(), String> {
    let fixture = super::super::tests::Fixture::new()?;
    let programs = super::super::tests::programs()?;
    let nifty = fixture.produce_span("NIFTY", &programs, 7, 8)?;
    let cash = fixture.produce_span("RELIANCE", &programs, 7, 8)?;
    assert_eq!(nifty.sessions().len(), 42);
    let expected = nifty.rows().len() + cash.rows().len();
    let procedure = PopulationStatisticsProcedureV2::new(7, 49, 2)?;
    let committed = produce(
        &fixture.output,
        vec![cash, nifty],
        procedure,
        stored_limits(),
    )?;
    committed.require_current()?;
    assert_eq!(committed.families().len(), 2);
    assert_eq!(
        committed
            .families()
            .first()
            .ok_or("first family")?
            .instrument()
            .underlying
            .as_str(),
        "NIFTY"
    );
    assert_eq!(committed.measurements().candidates.len(), expected);
    assert_eq!(committed.measurements().white.strategies(), expected);
    assert_eq!(committed.measurements().white.periods(), 42);
    assert_eq!(committed.layout().period_count(), 42);
    assert_eq!(
        committed.measurements().contributing_splits,
        committed.layout().split_count()
    );
    for measured in &committed.measurements().candidates {
        let row = committed
            .sources()
            .get(measured.family)
            .and_then(|source| source.rows().get(measured.coordinate))
            .ok_or("source coordinate")?;
        assert_eq!(measured.identity, row.identity());
        assert_eq!(
            (measured.return_paisa, measured.trades, measured.wins),
            (row.cell().pessimistic, row.cell().trades, row.cell().wins)
        );
        assert_eq!(
            measured.wilson_lower_bits,
            wilson_lower_bits(row.cell().wins, row.cell().trades)
        );
    }
    let body = std::fs::read(committed.directory.join("body.bin")).map_err(display)?;
    assert_eq!(body.len() as u64, wire::required_bytes(&committed.group)?);
    assert!(body.len().is_multiple_of(512));
    let again = produce(
        &fixture.output,
        vec![
            fixture.produce_span("NIFTY", &programs, 7, 8)?,
            fixture.produce_span("RELIANCE", &programs, 7, 8)?,
        ],
        procedure,
        stored_limits(),
    )?;
    assert_eq!(committed.identity(), again.identity());
    assert_eq!(committed.completion_digest(), again.completion_digest());
    assert_eq!(
        body,
        std::fs::read(again.directory.join("body.bin")).map_err(display)?
    );
    Ok(())
}

#[test]
fn committed_statistics_refuse_tampered_body_receipt_and_linked_candidate() -> Result<(), String> {
    let fixture = super::super::tests::Fixture::new()?;
    let programs = super::super::tests::programs()?;
    let family = fixture.produce_span("RELIANCE", &programs, 7, 8)?;
    let candidate_body = family.directory.join("body.bin");
    let committed = produce(
        &fixture.output,
        vec![family],
        PopulationStatisticsProcedureV2::new(7, 0, 2)?,
        stored_limits(),
    )?;
    for path in [
        committed.directory.join("body.bin"),
        committed.directory.join("complete.bin"),
        candidate_body,
    ] {
        let original = std::fs::read(&path).map_err(display)?;
        let mut changed = original.clone();
        *changed.last_mut().ok_or("fixture body")? ^= 1;
        std::fs::write(&path, changed).map_err(display)?;
        assert!(committed.require_current().is_err(), "{}", path.display());
        std::fs::write(&path, &original).map_err(display)?;
        committed.require_current()?;
        std::fs::write(
            &path,
            original.get(..original.len() - 1).ok_or("short body")?,
        )
        .map_err(display)?;
        assert!(
            committed.require_current().is_err(),
            "{} truncated",
            path.display()
        );
        std::fs::write(&path, original).map_err(display)?;
    }
    committed.require_current()?;
    Ok(())
}

#[test]
fn actual_source_odd_calendar_and_foreign_catalog_refuse_without_statistics_completion()
-> Result<(), String> {
    let fixture = super::super::tests::Fixture::new()?;
    let programs = super::super::tests::programs()?;
    let odd = fixture.produce("NIFTY", &programs)?;
    assert!(!odd.sessions().len().is_multiple_of(2));
    assert!(produce(&fixture.output, vec![odd], procedure()?, stored_limits()).is_err());
    assert!(!fixture.output.join("boolean-statistics-v1").exists());
    let nifty = fixture.produce_span("NIFTY", &programs, 7, 8)?;
    let different =
        fixture.produce_span("RELIANCE", programs.get(..1).ok_or("first program")?, 7, 8)?;
    assert!(
        produce(
            &fixture.output,
            vec![nifty, different],
            procedure()?,
            stored_limits()
        )
        .is_err()
    );
    assert!(!fixture.output.join("boolean-statistics-v1").exists());
    Ok(())
}
