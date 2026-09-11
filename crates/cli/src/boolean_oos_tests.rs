//! Generated complete-calendar stored comparison; no market-profitability claim.
use super::super::tests::Fixture;
use super::*;
fn config(fixture: &Fixture) -> Result<StrictConfig, String> {
    StrictConfig::from_values(
        Some(fixture.root.as_os_str().to_owned()),
        Some("4194304".into()),
        Some("40000".into()),
    )
    .map_err(display)
}
fn later<'a>(fixture: &'a Fixture, inputs: &'a StrictConfig) -> LaterRequest<'a> {
    LaterRequest {
        output: &fixture.output,
        inputs,
        from: (2025, 6),
        to: (2025, 6),
        bounds: Bounds {
            programs: 8,
            coordinates: 512,
            trades: 200_000,
            bytes: 32 * 1024 * 1024,
        },
    }
}
#[test]
fn stored_later_comparison_preserves_population_zero_sessions_and_pinned_cold_pages()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let programs = ["30 | !30", "30 & !30", "146 | !146"]
        .into_iter()
        .map(|s| Expression::parse(s).map_err(|e| format!("{e:?}")))
        .collect::<Result<Vec<_>, _>>()?;
    let training = fixture.produce("NIFTY", &programs)?;
    let inputs = config(&fixture)?;
    let observed = produce_identified(
        &training,
        later(&fixture, &inputs),
        "generated-boolean-candidate-fixture",
    )?;
    observed.require_current()?;
    assert_eq!(observed.rows().len(), training.rows().len());
    assert!(!observed.sessions().is_empty());
    assert!(observed.rows().iter().any(|r| r.cell.trades > 0));
    assert!(
        observed
            .rows()
            .iter()
            .filter(|r| r.program_index == 1)
            .all(|r| r.trades.is_empty()
                && r.periods
                    .iter()
                    .all(|p| p.trades == 0 && p.return_paisa == 0))
    );
    assert!(
        observed
            .rows()
            .iter()
            .filter(|r| r.program_index == 2)
            .all(|r| r.summary.unknown > 0 && r.cell.trades == 0)
    );
    let cold = reader::Reader::open(&fixture.output, observed.identity(), 64 * 1024 * 1024)?;
    assert_eq!(cold.summary().parent, training.identity());
    assert_eq!(
        cold.summary().parent_completion,
        training.completion_digest()
    );
    assert_eq!(cold.completion_digest(), observed.completion_digest());
    assert_eq!(cold.sessions(), observed.sessions());
    let admitted = cold
        .body_bytes()
        .checked_add(cold.parent().body_bytes())
        .and_then(|value| value.checked_add(224))
        .ok_or("reader total")?;
    assert!(reader::Reader::open(&fixture.output, observed.identity(), admitted).is_ok());
    assert!(reader::Reader::open(&fixture.output, observed.identity(), admitted - 1).is_err());
    let pin = cold.completion_digest();
    let rows = cold.coordinates(pin, 0, 256)?;
    assert_eq!(rows.len(), observed.rows().len().min(256));
    for (index, (actual, expected)) in rows.iter().zip(observed.rows()).enumerate() {
        assert_eq!(actual.cell, expected.cell);
        assert_eq!(actual.run, expected.run);
        assert_eq!(
            cold.trades(pin, index, 0, 256)?.len(),
            expected.trades.len().min(256)
        );
        assert_eq!(
            cold.periods(pin, index, 0, 256)?.len(),
            observed.sessions.len()
        );
    }
    assert!(cold.coordinates([0; 32], 0, 1).is_err());
    assert!(cold.coordinates(pin, 0, 257).is_err());
    assert!(cold.coordinates(pin, usize::MAX, 1).is_err());
    let retry = produce_identified(
        &training,
        later(&fixture, &inputs),
        "generated-boolean-candidate-fixture",
    )?;
    assert_eq!(retry.identity(), observed.identity());
    assert_eq!(retry.completion_digest(), observed.completion_digest());
    let body = observed.directory.join("body.bin");
    let mut bytes = std::fs::read(&body).map_err(display)?;
    refuse_malformed_codec(&bytes, observed.identity())?;
    let last = bytes.last_mut().ok_or("body")?;
    *last ^= 1;
    std::fs::write(&body, bytes).map_err(display)?;
    assert!(cold.require_current().is_err());
    assert!(reader::Reader::open(&fixture.output, observed.identity(), 64 * 1024 * 1024).is_err());
    Ok(())
}
fn refuse_malformed_codec(bytes: &[u8], identity: [u8; 32]) -> Result<(), String> {
    assert!(reader::decode(bytes, identity).is_ok());
    for (offset, value) in [(0, 0), (8, 0), (184, 0), (200, 13), (200, 5)] {
        let mut malformed = bytes.to_vec();
        if offset == 184 {
            malformed.get_mut(184..192).ok_or("count")?.fill(0);
        } else if offset == 8 {
            *malformed.get_mut(offset).ok_or("identity")? ^= 1;
        } else {
            *malformed.get_mut(offset).ok_or("field")? = value;
        }
        assert!(
            reader::decode(&malformed, identity).is_err(),
            "malformed field {offset}"
        );
    }
    assert!(reader::decode(bytes.get(..231).ok_or("header")?, identity).is_err());
    assert!(reader::decode(bytes.get(..bytes.len() - 1).ok_or("payload")?, identity).is_err());
    let mut appended = bytes.to_vec();
    appended.push(0);
    assert!(reader::decode(&appended, identity).is_err());
    let (summary, mut decoded) = reader::decode(bytes, identity)?;
    let last = decoded
        .4
        .iter_mut()
        .find(|row| row.cell.trades > 0)
        .ok_or("positive row")?
        .trades
        .last_mut()
        .ok_or("last trade")?;
    let day = (i128::from(last.exit_micros) + 19_800_000_000).div_euclid(86_400_000_000);
    last.exit_micros =
        i64::try_from(day * 86_400_000_000 + 54_540_000_000 - 19_800_000_000).map_err(display)?;
    assert!(
        reader::validate_actual(
            &decoded,
            summary.first_micros,
            summary.last_micros,
            summary.bars
        )
        .is_ok()
    );
    decoded
        .4
        .iter_mut()
        .find(|row| row.cell.trades > 0)
        .ok_or("positive row")?
        .trades
        .last_mut()
        .ok_or("last trade")?
        .exit_micros += 60_000_000;
    assert!(
        reader::validate_actual(
            &decoded,
            summary.first_micros,
            summary.last_micros,
            summary.bars
        )
        .is_err()
    );
    let (summary, mut decoded) = reader::decode(bytes, identity)?;
    let row = decoded
        .4
        .iter_mut()
        .find(|row| row.cell.trades > 0)
        .ok_or("positive row")?;
    let mut periods = row.periods.iter_mut();
    periods.next().ok_or("first period")?.return_paisa += 1;
    periods.next().ok_or("second period")?.return_paisa -= 1;
    assert!(
        reader::validate_actual(
            &decoded,
            summary.first_micros,
            summary.last_micros,
            summary.bars
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn stored_later_refuses_overlap_wrong_build_resource_and_original_receipt_loss()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let programs = [Expression::parse("30 | !30").map_err(|e| format!("{e:?}"))?];
    let training = fixture.produce("RELIANCE", &programs)?;
    let inputs = config(&fixture)?;
    let request = later(&fixture, &inputs);
    let mut overlap = request;
    overlap.from = (2025, 5);
    assert!(produce_identified(&training, overlap, "generated-boolean-candidate-fixture").is_err());
    assert!(produce_identified(&training, request, "other-build").is_err());
    let mut tiny = request;
    tiny.bounds.bytes = 1;
    let error = produce_identified(&training, tiny, "generated-boolean-candidate-fixture")
        .err()
        .ok_or("tiny cap should refuse")?;
    assert!(error.contains("physical ceiling"));
    std::fs::remove_file(training.directory.join("complete.bin")).map_err(display)?;
    assert!(produce_identified(&training, request, "generated-boolean-candidate-fixture").is_err());
    Ok(())
}

fn june_window(first: u8, last: u8) -> Result<LaterSessionWindowV1, String> {
    let day = |value| {
        pull::session::Day::new(2025, 6, value)
            .map(|day| i64::from(day.days_from_epoch()))
            .map_err(display)
    };
    LaterSessionWindowV1::new(day(first)?, day(last)?)
}

fn validation(windows: &[LaterSessionWindowV1]) -> Result<ValidationRequest<'_>, String> {
    Ok(ValidationRequest {
        requested: june_window(1, 30)?,
        windows,
        max_folds: 2,
        mapping_bytes: 1024 * 1024,
        projection_bytes: 1024 * 1024,
    })
}

fn verify_fold_projection(
    training: &CommittedBooleanFamilyV1,
    later: &CommittedBooleanOosV1<'_>,
) -> Result<(), String> {
    assert_eq!(later.training_identity(), training.identity());
    assert_eq!(
        later.training_completion_digest(),
        training.completion_digest()
    );
    assert_eq!(later.fold_projections().len(), training.rows().len());
    let mut measured = 0;
    let mut absent = 0;
    for ((original, row), proof) in training
        .rows()
        .iter()
        .zip(later.rows())
        .zip(later.fold_projections())
    {
        let Some(selected) = original.selected_exit() else {
            assert!(
                proof.is_none(),
                "a refused original cannot acquire fold authority"
            );
            absent += 1;
            continue;
        };
        let proof = proof
            .as_ref()
            .ok_or("selected original fold proof absent")?;
        measured += 1;
        assert_eq!(proof.selected_digest(), selected.digest());
        assert_eq!(proof.training_run_id(), original.run);
        assert_eq!(proof.later_run_id(), row.run);
        assert_eq!(proof.ordinal(), row.ordinal);
        assert_eq!(proof.execution_refusal_bits(), row.refusal);
        assert_eq!(proof.decided_folds(), 2);
        assert_eq!(proof.aggregate_oos_paisa()?, row.cell.pessimistic);
        let mut profitable = 0;
        for fold in proof.folds() {
            let periods = row
                .periods
                .iter()
                .filter(|period| {
                    period.day >= fold.window.first_day() && period.day <= fold.window.last_day()
                })
                .collect::<Vec<_>>();
            assert!(!periods.is_empty());
            assert_eq!(fold.sessions, periods.len() as u64);
            assert_eq!(
                fold.trades,
                periods.iter().map(|period| period.trades).sum::<u64>()
            );
            assert_eq!(
                fold.wins,
                periods.iter().map(|period| period.wins).sum::<u64>()
            );
            let expected = periods
                .iter()
                .map(|period| i128::from(period.return_paisa))
                .sum::<i128>();
            assert_eq!(i128::from(fold.return_paisa), expected);
            profitable += u64::from(expected > 0);
        }
        assert_eq!(proof.profitable_oos_folds(), profitable);
        let bytes = proof.canonical_bytes(1024)?;
        let observed = runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(&bytes, 1024, 2)?;
        assert_eq!(observed.canonical_bytes(1024)?, bytes);
    }
    assert!(
        measured > 0,
        "fixture must exercise actual selected coordinates"
    );
    assert!(
        absent > 0,
        "fixture must retain genuinely refused originals"
    );
    Ok(())
}

#[test]
fn optional_fixed_training_folds_preserve_every_v1_byte_and_exact_original_authority()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let programs = ["30 | !30", "30 & !30", "146 | !146"]
        .into_iter()
        .map(|program| Expression::parse(program).map_err(|why| format!("{why:?}")))
        .collect::<Result<Vec<_>, _>>()?;
    let training = fixture.produce("NIFTY", &programs)?;
    let inputs = config(&fixture)?;
    let request = later(&fixture, &inputs);
    let stamp = "generated-boolean-candidate-fixture";
    let ordinary = produce_identified(&training, request, stamp)?;
    assert_eq!(ordinary.fold_projections().len(), training.rows().len());
    assert!(ordinary.fold_projections().iter().all(Option::is_none));
    let body_path = ordinary.directory.join("body.bin");
    let before = std::fs::read(&body_path).map_err(display)?;
    let complete = std::fs::read(ordinary.directory.join("complete.bin")).map_err(display)?;
    let windows = [june_window(1, 15)?, june_window(16, 30)?];
    let validated =
        produce_with_validation_identified(&training, request, Some(validation(&windows)?), stamp)?;
    validated.require_current()?;
    let saved = reader::Reader::open(&fixture.output, validated.identity(), 64 * 1024 * 1024)?;
    assert_eq!(validated.source_identity(), saved.summary().source);
    assert_eq!(validated.identity(), ordinary.identity());
    assert_eq!(validated.completion_digest(), ordinary.completion_digest());
    assert_eq!(std::fs::read(&body_path).map_err(display)?, before);
    assert_eq!(
        std::fs::read(validated.directory.join("complete.bin")).map_err(display)?,
        complete
    );
    verify_fold_projection(&training, &validated)?;
    let changed = [june_window(1, 10)?, june_window(11, 30)?];
    let repartitioned =
        produce_with_validation_identified(&training, request, Some(validation(&changed)?), stamp)?;
    assert_eq!(repartitioned.identity(), validated.identity());
    assert_eq!(std::fs::read(&body_path).map_err(display)?, before);
    verify_fold_projection(&training, &repartitioned)?;
    for (first, second) in validated
        .fold_projections()
        .iter()
        .zip(repartitioned.fold_projections())
    {
        match (first, second) {
            (Some(first), Some(second)) => {
                assert_ne!(first.plan_digest(), second.plan_digest());
                assert_ne!(first.digest(1024)?, second.digest(1024)?);
                assert_eq!(first.aggregate_oos_paisa()?, second.aggregate_oos_paisa()?);
            }
            (None, None) => {}
            _ => return Err("repartition changed original authority availability".into()),
        }
    }
    std::fs::remove_file(training.directory.join("complete.bin")).map_err(display)?;
    assert!(validated.require_current().is_err());
    assert!(repartitioned.require_current().is_err());
    Ok(())
}

#[test]
fn optional_fold_proof_refuses_foreign_month_partitions_and_additional_memory_bounds()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let programs = [Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?];
    let training = fixture.produce("RELIANCE", &programs)?;
    let inputs = config(&fixture)?;
    let request = later(&fixture, &inputs);
    let windows = [june_window(1, 15)?, june_window(16, 30)?];
    let valid = validation(&windows)?;
    let stamp = "generated-boolean-candidate-fixture";
    let mut malformed = valid;
    malformed.requested = june_window(2, 30)?;
    assert!(
        produce_with_validation_identified(&training, request, Some(malformed), stamp)
            .err()
            .ok_or("date refusal")?
            .contains("complete later month")
    );
    malformed = valid;
    malformed.max_folds = 1;
    assert!(
        produce_with_validation_identified(&training, request, Some(malformed), stamp)
            .err()
            .ok_or("count refusal")?
            .contains("fold count")
    );
    let selected = training
        .rows()
        .iter()
        .filter(|row| row.selected_exit().is_some())
        .count();
    assert!(selected > 0);
    let exact = (training.rows().len() * size_of::<Option<FixedTrainingFoldProjectionV1>>()
        + selected * 2 * size_of::<FixedTrainingFoldV1>()
        + 2 * size_of::<LaterSessionWindowV1>()
        + size_of::<FixedTrainingFoldPlanV1>()) as u64;
    malformed = valid;
    malformed.projection_bytes = exact - 1;
    assert!(
        produce_with_validation_identified(&training, request, Some(malformed), stamp)
            .err()
            .ok_or("projection refusal")?
            .contains("retained projections")
    );
    malformed.projection_bytes = exact;
    malformed.mapping_bytes = 0;
    assert!(
        produce_with_validation_identified(&training, request, Some(malformed), stamp)
            .err()
            .ok_or("mapping refusal")?
            .contains("mapping exceeds")
    );
    let gapped = [june_window(1, 14)?, june_window(16, 30)?];
    malformed = valid;
    malformed.windows = &gapped;
    assert!(
        produce_with_validation_identified(&training, request, Some(malformed), stamp).is_err()
    );
    malformed = valid;
    malformed.projection_bytes = exact;
    let observed = produce_with_validation_identified(&training, request, Some(malformed), stamp)?;
    observed.require_current()?;
    assert_eq!(
        observed
            .fold_projections()
            .iter()
            .filter(|proof| proof.is_some())
            .count(),
        selected
    );
    Ok(())
}
