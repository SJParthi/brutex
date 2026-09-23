#![cfg(test)]
//! Pure codec fault probes with invented identities, never market authority.
use super::super::{BooleanCoordinateV1, base_values};
use super::*;
use crate::boolean_qualification_plan::{GeneratedBindings, Plan};
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use runner::bootstrap_zero_v2;

fn fixture() -> Result<(Manifest, Vec<Row>), String> {
    let policy = super::super::super::tests::policy_with_folds(u64::MAX, 2)?;
    let procedure = PopulationStatisticsProcedureV2::new(7, 49, 2)?;
    let programs =
        [runner::expression::Expression::parse("30 & !30").map_err(|why| format!("{why:?}"))?];
    let plan = Plan::generated_fixture(
        &policy,
        procedure,
        ((2025, 7), (2025, 8)),
        ((2025, 9), (2025, 9)),
        &programs,
        [64 * 1024 * 1024, 100_000],
        &GeneratedBindings {
            families: &[runner::research_family::ResearchFamilyV1::new(
                crate::stored::swept_index("NIFTY")?,
            )
            .map_err(display)?],
            catalogs: [[7; 32]; 8],
            sources: [[8; 32]; 8],
        },
    )?;
    let values = vec![vec![0; 3], vec![-1, 1, 2], vec![-3, -1, 2]];
    let numeric = plan.numerical_bounds()?;
    let receipt = bootstrap_zero_v2::evaluate(&values, 7, 49, 2, numeric)
        .map_err(|why| format!("{why:?}"))?;
    let white =
        runner::bootstrap::white_reality_check_receipt_v1(&values, 7, 49, 2).ok_or("White")?;
    let spa = runner::bootstrap::spa_receipt_v1(&values, 7, 49, 2).ok_or("SPA")?;
    let mut manifest = Manifest {
        identity: [0; 32],
        original: [1; 32],
        original_pin: [2; 32],
        scope: plan.descriptor(),
        unit: *plan.units().first().ok_or("unit")?,
        policy,
        procedure: [7, 49, 2],
        allocation: plan.allocation(0)?,
        bounds: Bounds {
            candidates: 100_000,
            observations: 100_000,
            work: numeric.max_work,
            memory: numeric.max_bytes,
            bytes: numeric.max_bytes,
        },
        folds: 2,
        count: 3,
        periods: 3,
        family_digest: receipt.family_digest(),
        shared_digest: receipt.shared_digest(),
        family_tests: [
            [
                white.statistic_bits(),
                white.p_value_bits(),
                white.exact_p_value().numerator() as u64,
                white.exact_p_value().denominator() as u64,
                white.matched_or_exceeded() as u64,
            ],
            [
                spa.statistic_bits(),
                spa.p_value_bits(),
                spa.exact_p_value().numerator() as u64,
                spa.exact_p_value().denominator() as u64,
                spa.matched_or_exceeded() as u64,
            ],
        ],
        later: vec![([3; 32], [4; 32], 3)],
        plan: plan.canonical_bytes()?,
    };
    manifest.identity = identity(&manifest);
    let projection = policy
        .evaluate_research_projection(base_values(&empty_coordinate()?)?)
        .map_err(|why| format!("{why:?}"))?;
    let rows = receipt
        .rows()
        .iter()
        .enumerate()
        .map(|(index, value)| Row {
            original: brutex_core::blake3::hash(&(index as u64).to_le_bytes()),
            later: [6; 32],
            family: 0,
            coordinate: index,
            projection,
            romano: super::super::numeric_words(*value),
            fold: None,
        })
        .collect();
    Ok((manifest, rows))
}

fn empty_coordinate() -> Result<BooleanCoordinateV1, String> {
    Ok(BooleanCoordinateV1 {
        identity: [8; 32],
        program_index: 0,
        run: [9; 32],
        side: runner::excursion::Side::Long,
        ordinal: 0,
        cell: runner::grid::Cell::default(),
        refusal: runner::exit_grid_policy::ExecutionRefusalBitsV1::from_bits(0).ok_or("refusal")?,
        periods: vec![],
        trades: vec![],
        selected: None,
        summary: runner::expression::Summary {
            evaluated: 0,
            hits: 0,
            misses: 0,
            unknown: 0,
        },
        support_sessions: 0,
    })
}

#[test]
fn qualification_codec_preserves_unreduced_zero_one_and_complete_active_mapping()
-> Result<(), String> {
    let (manifest, rows) = fixture()?;
    let raw = encode(&manifest, &rows)?;
    let (decoded, found) = decode(&raw, manifest.identity)?;
    assert_eq!(found.len(), 3);
    assert_eq!(
        found.first().ok_or("first")?.romano.get(..4),
        Some([2, 8, 8, 0].as_slice())
    );
    assert_eq!(decoded.family_digest, manifest.family_digest);
    assert_eq!(encode(&decoded, &found)?, raw);
    for (a, b) in rows.iter().zip(&found) {
        assert_eq!(a.romano, b.romano);
    }
    let mut wrong = rows.clone();
    wrong.get_mut(0).ok_or("zero")?.romano[1] = 1;
    wrong.get_mut(0).ok_or("zero")?.romano[2] = 1;
    assert!(
        encode(&manifest, &wrong).is_err(),
        "cannot normalize the exact denominator"
    );
    let positive = rows.get(1).ok_or("positive")?.romano;
    let negative = rows.get(2).ok_or("negative")?.romano;
    let mut swapped = rows;
    swapped.get_mut(1).ok_or("positive")?.romano = negative;
    swapped.get_mut(2).ok_or("negative")?.romano = positive;
    assert!(
        encode(&manifest, &swapped).is_err(),
        "per-row valid records cannot exchange source positions"
    );
    Ok(())
}

#[test]
fn qualification_codec_refuses_resealed_rank_probability_plan_and_padding_changes()
-> Result<(), String> {
    let (manifest, rows) = fixture()?;
    let raw = encode(&manifest, &rows)?;
    for offset in [
        0,
        8,
        900,
        HEADER + 8,
        HEADER + 16,
        HEADER + 24,
        HEADER + 432,
    ] {
        let mut changed = raw.clone();
        *changed.get_mut(offset).ok_or("offset")? ^= 1;
        assert!(decode(&changed, manifest.identity).is_err(), "byte{offset}");
    }
    for length in [0, 1023, raw.len() - 1] {
        assert!(decode(raw.get(..length).ok_or("length")?, manifest.identity).is_err());
    }
    let mut appended = raw.clone();
    appended.push(0);
    assert!(decode(&appended, manifest.identity).is_err());
    for edit in [
        |m: &mut Manifest| m.scope = [99; 32],
        |m: &mut Manifest| m.unit = [99; 32],
        |m: &mut Manifest| m.procedure[1] += 1,
        |m: &mut Manifest| m.bounds.work += 1,
        |m: &mut Manifest| m.bounds.memory += 1,
        |m: &mut Manifest| m.folds += 1,
        |m: &mut Manifest| m.family_tests[0][2] = 0,
        |m: &mut Manifest| m.family_tests[1][1] = f64::NAN.to_bits(),
        |m: &mut Manifest| m.shared_digest = None,
    ] {
        let mut changed = manifest.clone();
        edit(&mut changed);
        changed.identity = identity(&changed);
        assert!(encode(&changed, &rows).is_err());
    }
    for (index, word, value) in [
        (1, 4, 1),
        (1, 5, 1),
        (1, 6, f64::NAN.to_bits()),
        (1, 7, 8),
        (1, 8, 0),
        (1, 9, 9),
        (1, 10, 0),
        (1, 11, 9),
    ] {
        let mut changed = rows.clone();
        *changed
            .get_mut(index)
            .ok_or("row")?
            .romano
            .get_mut(word)
            .ok_or("word")? = value;
        assert!(encode(&manifest, &changed).is_err(), "RWfield{word}");
    }
    Ok(())
}

#[test]
fn qualification_codec_reconciles_exact_stepdown_recurrence_and_section_overflow()
-> Result<(), String> {
    let (mut manifest, rows) = fixture()?;
    let mut changed = rows;
    let stronger = changed.get_mut(1).ok_or("strong")?;
    assert_eq!(stronger.romano[5], 0);
    stronger.romano[1] = 1;
    stronger.romano[7] = 0;
    stronger.romano[8] = 1;
    stronger.romano[10] = 1;
    let weak = changed.get_mut(2).ok_or("weak")?;
    assert_eq!(weak.romano[5], 1);
    // Each scalar is valid, but2 cannot be the cumulative maximum of1 and1.
    weak.romano[7] = 0;
    weak.romano[8] = 1;
    weak.romano[10] = 2;
    assert!(encode(&manifest, &changed).is_err());
    manifest.count = usize::MAX;
    assert!(required_bytes(&manifest).is_err());
    assert!(proof_bytes(usize::MAX).is_err());
    Ok(())
}
