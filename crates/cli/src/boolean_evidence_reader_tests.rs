//! Generated complete-calendar fixtures; no arbitrary committed capability.
use super::super::super::tests::Fixture;
use super::super::{CommittedBooleanStatisticsV1, admission};
use super::*;
use brutex_core::blake3::hash;

const READ_BYTES: u64 = 256 * 1024 * 1024;

fn statistics(fixture: &Fixture) -> Result<CommittedBooleanStatisticsV1, String> {
    let mut programs = super::super::super::tests::programs()?;
    programs
        .push(runner::expression::Expression::parse("30 & !30").map_err(|why| format!("{why:?}"))?);
    let source = fixture.produce_span("RELIANCE", &programs, 7, 8)?;
    super::super::produce(
        &fixture.output,
        vec![source],
        PopulationStatisticsProcedureV2::new(7, 49, 2)?,
        super::super::tests::stored_limits(),
    )
}

#[test]
fn cold_statistics_pages_conserve_all_candidates_splits_sources_and_exact_read_budget()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let live = statistics(&fixture)?;
    let reader = Statistics::open(&fixture.output, live.identity(), READ_BYTES)?;
    reader.require_current()?;
    assert_eq!(reader.identity(), live.identity());
    assert_eq!(reader.completion_digest(), live.completion_digest());
    assert_eq!(reader.summary().periods, 42);
    assert_eq!(reader.summary().cohort, live.group.cohort);
    assert_eq!(reader.sources().len(), 1);
    let mut rows = Vec::new();
    for start in (0..reader.summary().candidates).step_by(17) {
        rows.extend(reader.rows(reader.completion_digest(), start, 17)?);
    }
    assert_eq!(rows.len(), live.measurements().candidates.len());
    for (actual, expected) in rows.iter().zip(&live.measurements().candidates) {
        assert_eq!(actual.identity, expected.identity);
        assert_eq!(
            (
                actual.trades,
                actual.wins,
                actual.return_paisa,
                actual.wilson_lower_bits
            ),
            (
                expected.trades,
                expected.wins,
                expected.return_paisa,
                expected.wilson_lower_bits
            )
        );
        assert_eq!(actual.period_digest, expected.period_digest);
    }
    let mut splits = Vec::new();
    for start in (0..reader.summary().splits).step_by(97) {
        splits.extend(reader.splits(reader.completion_digest(), start, 97)?);
    }
    assert_eq!(splits.len(), live.measurements().splits.len());
    for (actual, expected) in splits.iter().zip(&live.measurements().splits) {
        assert_eq!(
            (
                actual.train_mask,
                actual.test_mask,
                actual.bottom_half,
                actual.rankable,
                actual.scores_digest
            ),
            (
                expected.train_mask,
                expected.test_mask,
                expected.bottom_half,
                expected.rankable,
                expected.scores_digest
            )
        );
    }
    assert!(reader.rows([9; 32], 0, 1).is_err());
    for (start, limit) in [(0, 0), (0, 257), (usize::MAX, 1)] {
        assert!(
            reader
                .rows(reader.completion_digest(), start, limit)
                .is_err()
        );
    }
    assert!(
        reader
            .rows(reader.completion_digest(), rows.len(), 1)?
            .is_empty()
    );
    let charged = reader.admitted_bytes();
    assert!(Statistics::open(&fixture.output, live.identity(), charged - 1).is_err());
    assert_eq!(
        Statistics::open(&fixture.output, live.identity(), charged)?.admitted_bytes(),
        charged
    );
    Ok(())
}

#[test]
fn cold_admission_preserves_complete_policy_all_values_reasons_and_refuses_changed_ancestors()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let policy = admission::tests::policy(u64::MAX)?;
    let live = admission::produce(
        &fixture.output,
        statistics(&fixture)?,
        &policy,
        4 * 1024 * 1024,
    )?;
    let reader = admission::reader::Admission::open(&fixture.output, live.identity(), READ_BYTES)?;
    assert_eq!(reader.policy().canonical_bytes(), policy.canonical_bytes());
    assert_eq!(reader.row_count(), live.rows().len());
    let rows = reader.rows(reader.completion_digest(), 0, 256)?;
    assert_eq!(rows.len(), live.rows().len());
    for (actual, expected) in rows.iter().zip(live.rows()) {
        assert_eq!(actual.identity, expected.identity());
        assert_eq!(actual.source_index, expected.source_index());
        assert_eq!(actual.values, expected.values());
        assert_eq!(actual.verdict, expected.verdict());
        assert!(!actual.verdict.is_admitted());
    }
    assert!(reader.rows([9; 32], 0, 1).is_err());
    assert!(reader.rows(reader.completion_digest(), 0, 257).is_err());
    let source = reader.statistics().sources().first().ok_or("source")?;
    let path = fixture
        .output
        .join("boolean-candidates-v1")
        .join(crate::identity_hex(&source.identity))
        .join("body.bin");
    let original = std::fs::read(&path).map_err(display)?;
    let mut changed = original.clone();
    *changed.last_mut().ok_or("source last")? ^= 1;
    std::fs::write(&path, changed).map_err(display)?;
    assert!(reader.require_current().is_err());
    assert!(reader.rows(reader.completion_digest(), 0, 1).is_err());
    assert!(
        admission::reader::Admission::open(&fixture.output, live.identity(), READ_BYTES).is_err()
    );
    std::fs::write(&path, original).map_err(display)?;
    let restored =
        admission::reader::Admission::open(&fixture.output, live.identity(), READ_BYTES)?;
    restored.require_current()?;
    Ok(())
}

fn reseal(directory: &Path, body: &[u8]) -> Result<(), String> {
    let receipt_path = directory.join("complete.bin");
    let mut receipt = std::fs::read(&receipt_path).map_err(display)?;
    receipt
        .get_mut(40..72)
        .ok_or("payload slot")?
        .copy_from_slice(&hash(body));
    receipt
        .get_mut(72..80)
        .ok_or("length slot")?
        .copy_from_slice(&(body.len() as u64).to_le_bytes());
    let seal = hash(receipt.get(..80).ok_or("receipt prefix")?);
    receipt
        .get_mut(80..112)
        .ok_or("seal slot")?
        .copy_from_slice(&seal);
    std::fs::write(directory.join("body.bin"), body).map_err(display)?;
    std::fs::write(receipt_path, receipt).map_err(display)
}

#[test]
fn cold_statistics_refuses_resealed_padding_counts_kinds_and_cross_linked_sources()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let live = statistics(&fixture)?;
    let identity = live.identity();
    let directory = live.directory.clone();
    let original = std::fs::read(directory.join("body.bin")).map_err(display)?;
    for position in [144, 511, 512, 648, 680, 712, 1024 + 128] {
        let mut changed = original.clone();
        *changed.get_mut(position).ok_or("fixed attack byte")? ^= 128;
        reseal(&directory, &changed)?;
        assert!(
            Statistics::open(&fixture.output, identity, READ_BYTES).is_err(),
            "position {position}"
        );
    }
    let mut extra = original.clone();
    extra.extend_from_slice(&[0; 512]);
    reseal(&directory, &extra)?;
    assert!(Statistics::open(&fixture.output, identity, READ_BYTES).is_err());
    reseal(
        &directory,
        original.get(..original.len() - 512).ok_or("remove row")?,
    )?;
    assert!(Statistics::open(&fixture.output, identity, READ_BYTES).is_err());
    reseal(&directory, &original)?;
    Statistics::open(&fixture.output, identity, READ_BYTES)?.require_current()?;
    Ok(())
}

#[test]
fn cold_admission_refuses_resealed_policy_values_verdict_padding_and_foreign_stage()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let live = admission::produce(
        &fixture.output,
        statistics(&fixture)?,
        &admission::tests::policy(1)?,
        4 * 1024 * 1024,
    )?;
    let identity = live.identity();
    let directory = fixture
        .output
        .join("boolean-admission-v1")
        .join(crate::identity_hex(&identity));
    let original = std::fs::read(directory.join("body.bin")).map_err(display)?;
    for position in [40, 72, 104, 511, 512, 552, 592, 964, 1000] {
        let mut changed = original.clone();
        *changed.get_mut(position).ok_or("fixed admission attack")? ^= 128;
        reseal(&directory, &changed)?;
        assert!(
            admission::reader::Admission::open(&fixture.output, identity, READ_BYTES).is_err(),
            "position {position}"
        );
    }
    reseal(&directory, &original)?;
    assert!(Statistics::open(&fixture.output, identity, READ_BYTES).is_err());
    assert!(
        admission::reader::Admission::open(
            &fixture.output,
            live.statistics().identity(),
            READ_BYTES
        )
        .is_err()
    );
    admission::reader::Admission::open(&fixture.output, identity, READ_BYTES)?.require_current()?;
    Ok(())
}
