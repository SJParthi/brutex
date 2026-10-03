#![cfg(test)]
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

/// GAP14-59, D-1641: `produce` itself refuses through `admit` before any
/// attempt or numeric work, for each physical bound, over real committed
/// sources that the unrestricted bounds accept.
#[test]
fn produce_refuses_through_admit_before_any_attempt_or_statistics() -> Result<(), String> {
    let fixture = super::super::tests::Fixture::new()?;
    let programs = super::super::tests::programs()?;
    let procedure = PopulationStatisticsProcedureV2::new(7, 49, 2)?;
    let candidates = fixture.produce_span("NIFTY", &programs, 7, 8)?.rows().len() as u64;
    assert!(candidates > 1);
    let tight = [
        Bounds {
            candidates: candidates - 1,
            ..stored_limits()
        },
        Bounds {
            observations: 1,
            ..stored_limits()
        },
        Bounds {
            bootstrap_work: 1,
            ..stored_limits()
        },
        Bounds {
            split_work: 1,
            ..stored_limits()
        },
        Bounds {
            memory_bytes: 1,
            ..stored_limits()
        },
        Bounds {
            bytes: 1,
            ..stored_limits()
        },
    ];
    let mut sources = Vec::new();
    for _ in &tight {
        sources.push(fixture.produce_span("NIFTY", &programs, 7, 8)?);
    }
    let evidence = fixture.output.join("results").join("sweep-evidence-v1");
    let before = std::fs::read_dir(&evidence).map_or(0, Iterator::count);
    for (bounds, source) in tight.into_iter().zip(sources) {
        let refusal = produce(&fixture.output, vec![source], procedure, bounds)
            .err()
            .ok_or("a bound below the work must refuse")?;
        assert_eq!(
            refusal,
            "Boolean statistics complete-family work/memory admission refused"
        );
        assert!(!fixture.output.join("boolean-statistics-v1").exists());
    }
    assert_eq!(
        std::fs::read_dir(&evidence).map_or(0, Iterator::count),
        before,
        "admission refuses before any sweep-evidence attempt is begun"
    );
    let committed = produce(
        &fixture.output,
        vec![fixture.produce_span("NIFTY", &programs, 7, 8)?],
        procedure,
        stored_limits(),
    )?;
    committed.require_current()?;
    Ok(())
}

/// The body of the first `require_current` method that follows `after` in
/// `source`, up to its closing brace at method indentation.
fn require_current_body<'a>(source: &'a str, after: &str) -> Result<&'a str, String> {
    let from = source
        .find(after)
        .ok_or_else(|| format!("`{after}` is not in the source"))?;
    let rest = source.get(from..).ok_or("source boundary")?;
    let open = rest
        .find("fn require_current(&self) -> Result<(), String> {")
        .ok_or_else(|| format!("no require_current after `{after}`"))?;
    let body = rest.get(open..).ok_or("method boundary")?;
    let close = body
        .find("\n    }\n")
        .ok_or("method has no closing brace")?;
    body.get(..close)
        .ok_or_else(|| "method body boundary".to_owned())
}

/// W2-cli2-4: `docs/06-limits.md` states that every Boolean integrity check
/// re-reads and re-hashes its whole body and checks its parent before and
/// after, so a family body's reads double with each level above it. Each
/// line the entry quotes is found in the entry and in the source that pays
/// it, and each level's bracketing is counted in its own method: two parent
/// checks around exactly one `persistence::verify`.
#[test]
fn the_boolean_integrity_cost_entry_is_read_off_the_source() -> Result<(), String> {
    let limits = include_str!("../../../docs/06-limits.md");
    let start = limits
        .find("## Boolean integrity checks re-read whole bodies, and nesting doubles them")
        .ok_or("the W2-cli2-4 limits entry is missing")?;
    let entry = limits.get(start..).ok_or("entry boundary")?;
    let entry = entry
        .get(3..)
        .and_then(|tail| tail.find("\n## "))
        .and_then(|end| entry.get(..end.saturating_add(3)))
        .unwrap_or(entry);
    assert!(entry.contains("W2-cli2-4"), "{entry}");
    assert!(
        entry.contains("**Not O(1), and not fixed here.**"),
        "{entry}"
    );

    let persistence = include_str!("boolean_candidate_persistence.rs");
    for quote in [
        "let body = read_exact(&directory.join(\"body.bin\"), bytes)?;",
        "hash(&body) != payload",
    ] {
        assert!(entry.contains(quote), "the entry quotes `{quote}`");
        assert!(persistence.contains(quote), "the source pays `{quote}`");
    }

    let statistics = include_str!("boolean_statistics_v1.rs");
    let admission = include_str!("boolean_admission_v1.rs");
    let oos = include_str!("boolean_oos_v1.rs");
    let qualification = include_str!("boolean_qualification_v1.rs");
    for (source, owner, parent) in [
        (
            statistics,
            "impl CommittedBooleanStatisticsV1",
            "self.group.require_current()",
        ),
        (
            admission,
            "impl CommittedBooleanAdmissionV1",
            "self.statistics.require_current()",
        ),
        (
            oos,
            "impl CommittedBooleanOosV1<'_>",
            "self.training.require_current()",
        ),
        (
            qualification,
            "impl Committed<'_> {",
            "current(self.training, self.later)",
        ),
    ] {
        assert!(entry.contains(parent), "the entry names `{parent}`");
        let body = require_current_body(source, owner)?;
        assert_eq!(
            body.matches(parent).count(),
            2,
            "{owner} checks its parent before and after: {body}"
        );
        assert_eq!(
            body.matches("persistence::verify(").count(),
            1,
            "{owner} re-reads its own body once: {body}"
        );
    }
    // The fan-out below each bracket: the statistics group checks every
    // source family, and qualification's `current` checks the admission and
    // every out-of-sample source.
    let group = require_current_body(statistics, "impl Group")?;
    assert!(group.contains("for source in &self.sources"), "{group}");
    assert!(group.contains("source.require_current()?;"), "{group}");
    let current = qualification
        .find("\nfn current(")
        .and_then(|at| qualification.get(at..))
        .and_then(|tail| tail.find("\n}\n").and_then(|end| tail.get(..end)))
        .ok_or("qualification's `current` is missing")?;
    assert!(
        current.contains("training.require_current()?;"),
        "{current}"
    );
    assert!(current.contains("for source in later"), "{current}");
    assert!(current.contains("source.require_current()?;"), "{current}");
    let family = include_str!("boolean_candidate_v1.rs");
    let family = require_current_body(family, "impl CommittedBooleanFamilyV1")?;
    assert_eq!(
        family.matches("persistence::verify(").count(),
        1,
        "a family check re-reads the family body: {family}"
    );
    Ok(())
}
