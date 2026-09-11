//! Shared numerical kernels adapted to complete native stop observations.
use super::{
    DailyRecord, Facts, Fold, ReplayWork, Row, Snapshot, Split, Statistics, daily, display,
};
use crate::index_consistency::{self, Policy, Session};
use crate::population_observations_v1::{canonical_masks, derive_layout};
use crate::population_statistics_v2::{cscv_placement, wilson_lower_bits};
use brutex_core::blake3::Hasher;
use runner::admission::research_projection::{hypothesis_decision, wilson_ppm};
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionExactProbabilityV2, CompletenessV1, ObservedI64V1,
    ObservedU64V1,
};
use runner::bootstrap::{spa_receipt_v1, white_reality_check_receipt_v1};
use runner::bootstrap_zero_v2::{self as zero, Classification};

#[path = "index_stop_qualification_metrics.rs"]
mod metrics;

pub(super) fn validate_family<S: Snapshot>(
    training: &[S],
    later: &[S],
    facts: &Facts,
) -> Result<(), String> {
    if facts.search == [0; 32]
        || facts.bounds.words().contains(&0)
        || training.is_empty()
        || training.len() != facts.count
        || training.len() != later.len()
        || !training.len().is_multiple_of(2)
        || u64::try_from(training.len()).map_err(display)? > facts.bounds.candidates
    {
        return Err("single-stop qualification requires every complete program/side coordinate within admission".into());
    }
    facts
        .allocation
        .require_draws(facts.procedure.draws())
        .map_err(|why| format!("single-stop allocation draw resolution refused: {why:?}"))?;
    let first = training
        .first()
        .ok_or("single-stop training family missing")?;
    let after_first = later.first().ok_or("single-stop later family missing")?;
    let rung = crate::EVERY_RUNG
        .iter()
        .position(|label| *label == first.timeframe())
        .ok_or("single-stop unsupported timeframe")?;
    if facts.allocation.rung() != u64::try_from(rung).map_err(display)?
        || first.family().legacy_index_family().is_none()
        || first.first() > first.last()
        || after_first.first() > after_first.last()
        || first.last() >= after_first.first()
    {
        return Err(
            "single-stop qualification changes index, timeframe, allocation or ordered day windows"
                .into(),
        );
    }
    for (index, (before, after)) in training.iter().zip(later).enumerate() {
        let direction = if index.is_multiple_of(2) {
            runner::identity::Direction::Long
        } else {
            runner::identity::Direction::Short
        };
        if before.direction() != direction
            || after.direction() != direction
            || before.program() != after.program()
            || before.family() != first.family()
            || after.family() != first.family()
            || before.timeframe() != first.timeframe()
            || after.timeframe() != first.timeframe()
            || before.first() != first.first()
            || before.last() != first.last()
            || after.first() != after_first.first()
            || after.last() != after_first.last()
            || before.source() != first.source()
            || after.source() != after_first.source()
            || before.periods().len() != first.periods().len()
            || after.periods().len() != after_first.periods().len()
            || before
                .periods()
                .iter()
                .zip(first.periods())
                .any(|(a, b)| a.day != b.day)
            || after
                .periods()
                .iter()
                .zip(after_first.periods())
                .any(|(a, b)| a.day != b.day)
        {
            return Err(
                "single-stop training/later complete coordinates or session geometry differ".into(),
            );
        }
        if index % 2 == 1
            && training
                .get(index - 1)
                .is_none_or(|pair| pair.program() != before.program())
        {
            return Err("single-stop long/short program pairing differs".into());
        }
    }
    admit_memory(training, later, facts)
}

pub(super) fn required_work<S: Snapshot>(
    training: &[S],
    later: &[S],
    facts: &Facts,
) -> Result<ReplayWork, String> {
    let training_periods = training
        .first()
        .ok_or("single-stop training source missing")?
        .periods()
        .len();
    let split_work = if let Ok(layout) = derive_layout(training_periods) {
        u128::from(layout.split_count())
            .checked_mul(facts.count as u128)
            .and_then(|n| n.checked_mul(training_periods as u128))
            .ok_or("single-stop CSCV work overflow")?
    } else {
        0
    };
    let periods = later
        .first()
        .ok_or("single-stop later source missing")?
        .periods()
        .len() as u128;
    let bootstrap_work = (facts.count as u128)
        .checked_mul(periods)
        .and_then(|n| n.checked_mul(u128::from(facts.procedure.draws()).checked_add(1)?))
        .and_then(|n| n.checked_mul(3))
        .ok_or("single-stop complete bootstrap work overflow")?;
    Ok(ReplayWork {
        bootstrap_work: u64::try_from(bootstrap_work)
            .map_err(|_| "single-stop complete bootstrap work exceeds representable admission")?,
        split_work: u64::try_from(split_work)
            .map_err(|_| "single-stop CSCV work exceeds representable admission")?,
    })
}
fn admit_memory<S: Snapshot>(training: &[S], later: &[S], facts: &Facts) -> Result<(), String> {
    let work = required_work(training, later, facts)?;
    if work.split_work > facts.bounds.split_work {
        return Err("single-stop complete training CSCV exceeds work admission".into());
    }
    if work.bootstrap_work > facts.bounds.bootstrap_work {
        return Err(
            "single-stop White, SPA and Romano-Wolf bootstrap work exceeds combined admission"
                .into(),
        );
    }
    if required_memory(training, later, facts)? > u128::from(facts.bounds.memory_bytes) {
        return Err("single-stop complete numeric/source memory exceeds admission".into());
    }
    Ok(())
}

/// Payload estimate shared by production and cold replay, including the saved
/// split vector that a cold reader retains. It excludes allocator metadata,
/// capacity over-allocation, thread stacks and unrelated process RSS.
pub(super) fn required_memory<S: Snapshot>(
    training: &[S],
    later: &[S],
    facts: &Facts,
) -> Result<u128, String> {
    let mut rows = 0_u128;
    let mut bytes = 0_u128;
    for row in training.iter().chain(later) {
        rows = rows
            .checked_add(row.periods().len() as u128)
            .ok_or("single-stop source row-count overflow")?;
        bytes = bytes
            .checked_add(
                (row.events().len() as u128)
                    .checked_mul(112)
                    .and_then(|n| n.checked_add((row.trades().len() as u128).checked_mul(168)?))
                    .and_then(|n| n.checked_add((row.periods().len() as u128).checked_mul(96)?))
                    .ok_or("single-stop source payload memory overflow")?,
            )
            .ok_or("single-stop source payload memory overflow")?;
    }
    // Includes retained raw parents, return matrices, daily payloads and bounded
    // output working copies. Bootstrap's own buffers are admitted separately.
    let days = later
        .first()
        .ok_or("single-stop later source missing")?
        .last()
        .checked_sub(
            training
                .first()
                .ok_or("single-stop training source missing")?
                .first(),
        )
        .and_then(|n| n.checked_add(1))
        .and_then(|n| u128::try_from(n).ok())
        .ok_or("single-stop span overflow")?;
    let periods = later
        .first()
        .ok_or("single-stop later source missing")?
        .periods()
        .len() as u128;
    let draws = u128::from(facts.procedure.draws());
    let bootstrap_bytes = draws
        .checked_mul(periods)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_add(draws.checked_mul(32)?))
        .and_then(|n| n.checked_add((facts.count as u128).checked_mul(1024)?))
        .ok_or("single-stop bootstrap memory overflow")?;
    let training_periods = training
        .first()
        .ok_or("single-stop training source missing")?
        .periods()
        .len();
    let cscv_bytes = if let Ok(layout) = derive_layout(training_periods) {
        // The consuming mask iterator keeps its allocation while reproduced
        // splits accumulate. A cold reader also keeps the original Split vector.
        // Two candidate-sized score vectors overlap with all three payloads.
        u128::from(layout.split_count())
            .checked_mul(
                (std::mem::size_of::<Split>() as u128)
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(std::mem::size_of::<(u64, u64)>() as u128))
                    .ok_or("single-stop CSCV element memory overflow")?,
            )
            .and_then(|n| {
                n.checked_add(
                    (facts.count as u128)
                        .checked_mul(2)?
                        .checked_mul(std::mem::size_of::<i64>() as u128)?,
                )
            })
            .ok_or("single-stop CSCV payload memory overflow")?
    } else {
        0
    };
    bytes
        .checked_add(
            rows.checked_mul(128)
                .ok_or("single-stop row-memory overflow")?,
        )
        .and_then(|n| n.checked_add((facts.count as u128).checked_mul(days)?.checked_mul(128)?))
        .and_then(|n| n.checked_add(bootstrap_bytes))
        .and_then(|n| n.checked_add(cscv_bytes))
        .ok_or_else(|| "single-stop numeric memory overflow".into())
}

/// A stored extent cannot allocate more replay-retained splits than the
/// independently derived geometry charged by `required_memory`.
pub(super) fn validate_saved_layout<S: Snapshot>(
    training: &[S],
    statistics: &Statistics,
) -> Result<(), String> {
    let periods = training
        .first()
        .ok_or("single-stop training source missing")?
        .periods()
        .len();
    let (expected, digest, count) = match derive_layout(periods) {
        Ok(layout) => (
            Some([
                layout.period_count(),
                u64::from(layout.segment_count()),
                layout.periods_per_segment(),
                layout.split_count(),
            ]),
            layout.digest(),
            layout.split_count(),
        ),
        Err(_) => (None, [0; 32], 0),
    };
    if statistics.layout != expected
        || statistics.layout_digest != digest
        || u64::try_from(statistics.splits.len()).map_err(display)? != count
    {
        return Err(
            "single-stop saved CSCV layout or split extent differs from exact training geometry"
                .into(),
        );
    }
    Ok(())
}

pub(super) fn measure<S: Snapshot>(
    training: &[S],
    later: &[S],
    facts: &Facts,
) -> Result<(Statistics, Vec<Row>), String> {
    validate_family(training, later, facts)?;
    let train_returns = returns(training)?;
    let (layout, layout_digest, splits) = cscv(&train_returns, facts)?;
    drop(train_returns);
    let returns = returns(later)?;
    let draws = usize::try_from(facts.procedure.draws()).map_err(display)?;
    let block = usize::try_from(facts.procedure.block_length()).map_err(display)?;
    let romano = zero::evaluate(
        &returns,
        draws,
        facts.procedure.seed(),
        block,
        zero::Bounds {
            max_work: facts.bounds.bootstrap_work,
            max_bytes: facts.bounds.memory_bytes,
        },
    )
    .map_err(|why| format!("single-stop complete later Romano-Wolf procedure refused: {why:?}"))?;
    let white = white_reality_check_receipt_v1(&returns, draws, facts.procedure.seed(), block)
        .ok_or("single-stop later White procedure refused")?;
    let spa = spa_receipt_v1(&returns, draws, facts.procedure.seed(), block)
        .ok_or("single-stop later SPA procedure refused")?;
    let statistics = Statistics {
        white: [
            white.statistic_bits(),
            white.p_value_bits(),
            u64::try_from(white.exact_p_value().numerator()).map_err(display)?,
            u64::try_from(white.exact_p_value().denominator()).map_err(display)?,
            u64::try_from(white.matched_or_exceeded()).map_err(display)?,
        ],
        spa: [
            spa.statistic_bits(),
            spa.p_value_bits(),
            u64::try_from(spa.exact_p_value().numerator()).map_err(display)?,
            u64::try_from(spa.exact_p_value().denominator()).map_err(display)?,
            u64::try_from(spa.matched_or_exceeded()).map_err(display)?,
        ],
        family: romano.family_digest(),
        shared: romano.shared_digest(),
        layout,
        layout_digest,
        splits,
    };
    drop(returns);
    let mut rows = Vec::new();
    rows.try_reserve_exact(facts.count).map_err(display)?;
    for (index, (before, after)) in training.iter().zip(later).enumerate() {
        let romano = numeric(
            *romano
                .candidate(index)
                .ok_or("single-stop adjusted probability omitted a coordinate")?,
        )?;
        let folds = folds(after, facts)?;
        let wilson_bits = wilson_lower_bits(after.metrics().wins, after.metrics().trades);
        let values = project(
            before,
            after,
            facts,
            &statistics,
            romano,
            &folds,
            wilson_bits,
        )?;
        rows.push(Row {
            original_run: before.run(),
            later_run: after.run(),
            original_digest: before.digest(),
            later_digest: after.digest(),
            projection: facts
                .policy
                .evaluate_research_projection(values)
                .map_err(|why| format!("single-stop institutional projection refused: {why:?}"))?,
            wilson_bits,
            romano,
            folds,
        });
    }
    Ok((statistics, rows))
}

fn returns<S: Snapshot>(sources: &[S]) -> Result<Vec<Vec<i64>>, String> {
    let mut result = Vec::new();
    result.try_reserve_exact(sources.len()).map_err(display)?;
    for row in sources {
        let mut days = Vec::new();
        days.try_reserve_exact(row.periods().len())
            .map_err(display)?;
        let mut sum = (0_i64, 0_u64, 0_u64);
        for day in row.periods() {
            sum.0 = sum
                .0
                .checked_add(day.pessimistic_paisa)
                .ok_or("single-stop day money overflow")?;
            sum.1 = sum
                .1
                .checked_add(day.trades)
                .ok_or("single-stop day trades overflow")?;
            sum.2 = sum
                .2
                .checked_add(day.wins)
                .ok_or("single-stop day wins overflow")?;
            days.push(day.pessimistic_paisa);
        }
        if sum
            != (
                row.metrics().pessimistic_paisa,
                row.metrics().trades,
                row.metrics().wins,
            )
        {
            return Err("single-stop day sums do not reconcile native totals".into());
        }
        result.push(days);
    }
    Ok(result)
}

type CscvEvidence = (Option<[u64; 4]>, [u8; 32], Vec<Split>);

fn cscv(returns: &[Vec<i64>], facts: &Facts) -> Result<CscvEvidence, String> {
    let periods = returns
        .first()
        .ok_or("single-stop training family empty")?
        .len();
    let Ok(layout) = derive_layout(periods) else {
        return Ok((None, [0; 32], Vec::new()));
    };
    let work = u128::from(layout.split_count())
        .checked_mul(facts.count as u128)
        .and_then(|n| n.checked_mul(periods as u128))
        .ok_or("single-stop CSCV work overflow")?;
    if work > u128::from(facts.bounds.split_work) {
        return Err("single-stop complete training CSCV exceeds work admission".into());
    }
    let masks = canonical_masks(layout)?;
    let mut splits = Vec::new();
    splits.try_reserve_exact(masks.len()).map_err(display)?;
    let width = usize::try_from(layout.periods_per_segment()).map_err(display)?;
    for (train, test) in masks {
        let mut trains = Vec::new();
        let mut tests = Vec::new();
        trains.try_reserve_exact(facts.count).map_err(display)?;
        tests.try_reserve_exact(facts.count).map_err(display)?;
        let mut digest = Hasher::new();
        digest.update(b"brutex-index-stop-training-cscv-v1\0");
        digest.update(&layout.digest());
        digest.update(&train.to_le_bytes());
        digest.update(&test.to_le_bytes());
        for row in returns {
            let mut sums = (0_i64, 0_i64);
            for (index, &value) in row.iter().enumerate() {
                let bit = 1_u64
                    .checked_shl(u32::try_from(index / width).map_err(display)?)
                    .ok_or("single-stop CSCV mask overflow")?;
                let sum = if train & bit != 0 {
                    &mut sums.0
                } else if test & bit != 0 {
                    &mut sums.1
                } else {
                    return Err("single-stop CSCV leaves an observation out".into());
                };
                *sum = sum
                    .checked_add(value)
                    .ok_or("single-stop CSCV money overflow")?;
            }
            trains.push(sums.0);
            tests.push(sums.1);
            digest.update(&sums.0.to_le_bytes());
            digest.update(&sums.1.to_le_bytes());
        }
        let (bottom_half, rankable) = cscv_placement(&trains, &tests)?;
        splits.push(Split {
            train,
            test,
            bottom_half,
            rankable,
            scores: digest.finalize(),
        });
    }
    Ok((
        Some([
            layout.period_count(),
            u64::from(layout.segment_count()),
            layout.periods_per_segment(),
            layout.split_count(),
        ]),
        layout.digest(),
        splits,
    ))
}

fn numeric(candidate: zero::Candidate) -> Result<[u64; 12], String> {
    let p = candidate.p_value();
    let mut result = [0; 12];
    result[0] = match candidate.classification() {
        Classification::MeasuredPositive => 1,
        Classification::ConservativeZero => 2,
        Classification::ConservativeNonpositive => 3,
    };
    result[1] = p.numerator();
    result[2] = p.denominator();
    if let Some(shared) = candidate.shared() {
        result[3] = 1;
        result[4..].copy_from_slice(&[
            u64::try_from(shared.strategy()).map_err(display)?,
            u64::try_from(shared.stepdown_rank()).map_err(display)?,
            shared.observed_statistic().to_bits(),
            u64::try_from(shared.strict_exceedances()).map_err(display)?,
            u64::try_from(shared.initial_p_value().numerator()).map_err(display)?,
            u64::try_from(shared.initial_p_value().denominator()).map_err(display)?,
            u64::try_from(shared.adjusted_p_value().numerator()).map_err(display)?,
            u64::try_from(shared.adjusted_p_value().denominator()).map_err(display)?,
        ]);
    }
    Ok(result)
}

fn project<S: Snapshot>(
    before: &S,
    after: &S,
    facts: &Facts,
    statistics: &Statistics,
    romano: [u64; 12],
    folds: &[Fold],
    wilson: u64,
) -> Result<AdmissionEvidenceValuesV1, String> {
    let mut values = metrics::base(after)?;
    values.wilson_win_rate_ppm = ObservedU64V1::Measured(
        wilson_ppm(after.metrics().wins, after.metrics().trades, wilson)
            .ok_or("single-stop Wilson input disagrees")?,
    );
    let contributing =
        u64::try_from(statistics.splits.iter().filter(|s| s.rankable).count()).map_err(display)?;
    let bottom = u64::try_from(
        statistics
            .splits
            .iter()
            .filter(|s| s.rankable && s.bottom_half)
            .count(),
    )
    .map_err(display)?;
    if statistics.layout.is_some() {
        values.pbo_contributing_folds = ObservedU64V1::Measured(contributing);
        values.pbo_unrankable_folds = ObservedU64V1::Measured(
            u64::try_from(statistics.splits.len()).map_err(display)? - contributing,
        );
        if contributing > 0 {
            values.pbo_ppm = ObservedU64V1::Measured(probability(bottom, contributing)?.ppm());
        }
    }
    let adjusted = scaled(facts, romano[1], romano[2])?;
    values.fwer_p_value_ppm = ObservedU64V1::Measured(upper(adjusted)?);
    values.romano_wolf_p_value_ppm = ObservedU64V1::Measured(upper(adjusted)?);
    values.romano_wolf_decision = hypothesis_decision(adjusted);
    let white = scaled(facts, statistics.white[2], statistics.white[3])?;
    values.white_reality_p_value_ppm = ObservedU64V1::Measured(upper(white)?);
    values.white_reality_decision = hypothesis_decision(white);
    values.spa_p_value_ppm =
        ObservedU64V1::Measured(upper(scaled(facts, statistics.spa[2], statistics.spa[3])?)?);
    values.bootstrap_draws = ObservedU64V1::Measured(facts.procedure.draws());
    values.bootstrap_strategies =
        ObservedU64V1::Measured(u64::try_from(facts.count).map_err(display)?);
    values.bootstrap_periods =
        ObservedU64V1::Measured(u64::try_from(after.periods().len()).map_err(display)?);
    values.decided_folds = ObservedU64V1::Measured(
        u64::try_from(folds.iter().filter(|f| f.decided).count()).map_err(display)?,
    );
    values.profitable_oos_folds = ObservedU64V1::Measured(
        u64::try_from(
            folds
                .iter()
                .filter(|f| f.decided && f.pessimistic_paisa > 0)
                .count(),
        )
        .map_err(display)?,
    );
    values.oos_pessimistic_return_paisa =
        ObservedI64V1::Measured(after.metrics().pessimistic_paisa);
    values.full_precision_statistics_complete = if statistics.layout.is_some() && contributing > 0 {
        CompletenessV1::Complete
    } else {
        CompletenessV1::Unmeasured
    };
    if !metrics::execution_complete(before) {
        values.execution_complete = CompletenessV1::Refused;
    }
    let calendar = assessment(before, false)?;
    if calendar.summary.missing_days > 0 || calendar.summary.unmeasured_days > 0 {
        values.calendar_complete = CompletenessV1::Refused;
    }
    Ok(values)
}
fn probability(n: u64, d: u64) -> Result<AdmissionExactProbabilityV2, String> {
    AdmissionExactProbabilityV2::new(n, d)
        .map_err(|why| format!("single-stop exact probability refused: {why:?}"))
}
fn scaled(facts: &Facts, n: u64, d: u64) -> Result<AdmissionExactProbabilityV2, String> {
    let p = facts
        .allocation
        .scaled_probability(n, d)
        .map_err(|why| format!("single-stop probability allocation refused: {why:?}"))?;
    probability(
        u64::try_from(p.numerator()).map_err(display)?,
        u64::try_from(p.denominator()).map_err(display)?,
    )
}
fn upper(p: AdmissionExactProbabilityV2) -> Result<u64, String> {
    u64::try_from((u128::from(p.numerator()) * 1_000_000).div_ceil(u128::from(p.denominator())))
        .map_err(display)
}

fn folds<S: Snapshot>(row: &S, facts: &Facts) -> Result<Vec<Fold>, String> {
    let policy = facts.policy.values();
    let count = policy
        .min_decided_folds
        .max(policy.min_profitable_oos_folds);
    let days = row
        .last()
        .checked_sub(row.first())
        .and_then(|n| n.checked_add(1))
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("single-stop fold span overflow")?;
    if count == 0 || count > days || count > facts.bounds.bytes / 64 {
        return Err("single-stop fixed fold count exceeds later day span or byte admission".into());
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(usize::try_from(count).map_err(display)?)
        .map_err(display)?;
    let mut first = row.first();
    let mut at = 0;
    for index in 0..count {
        let width = days / count + u64::from(index < days % count);
        let last = first
            .checked_add(i64::try_from(width - 1).map_err(display)?)
            .ok_or("single-stop fold boundary overflow")?;
        let start = at;
        while row.periods().get(at).is_some_and(|p| p.day <= last) {
            at += 1;
        }
        let periods = row
            .periods()
            .get(start..at)
            .ok_or("single-stop fold period extent differs")?;
        let sessions = sessions(periods)?;
        let calendar =
            index_consistency::evaluate(Policy::V1, row.family(), first, last, &sessions, false);
        let mut fold = Fold {
            first_day: first,
            last_day: last,
            observed_days: u64::try_from(periods.len()).map_err(display)?,
            trades: 0,
            wins: 0,
            pessimistic_paisa: 0,
            refused: 0,
            decided: !periods.is_empty()
                && calendar.summary.missing_days == 0
                && calendar.summary.unmeasured_days == 0
                && periods
                    .iter()
                    .all(|p| p.unavailable_minutes == 0 && p.close_verified && p.refused == 0),
        };
        for p in periods {
            fold.trades = fold
                .trades
                .checked_add(p.trades)
                .ok_or("single-stop fold count overflow")?;
            fold.wins = fold
                .wins
                .checked_add(p.wins)
                .ok_or("single-stop fold wins overflow")?;
            fold.pessimistic_paisa = fold
                .pessimistic_paisa
                .checked_add(p.pessimistic_paisa)
                .ok_or("single-stop fold return overflow")?;
            fold.refused = fold
                .refused
                .checked_add(p.refused)
                .ok_or("single-stop fold refusal overflow")?;
        }
        result.push(fold);
        first = last
            .checked_add(1)
            .ok_or("single-stop next fold overflow")?;
    }
    if at != row.periods().len() {
        return Err("single-stop folds omitted native later periods".into());
    }
    Ok(result)
}

fn sessions(periods: &[runner::signal_candle_stop::Period]) -> Result<Vec<Session>, String> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(periods.len()).map_err(display)?;
    rows.extend(periods.iter().map(|p| Session {
        day: p.day,
        pessimistic_paisa: p.pessimistic_paisa,
        optimistic_paisa: p.optimistic_paisa,
        trades: p.trades,
    }));
    Ok(rows)
}
fn assessment<S: Snapshot>(row: &S, weeks: bool) -> Result<index_consistency::Evaluation, String> {
    let rows = sessions(row.periods())?;
    Ok(index_consistency::evaluate(
        Policy::V1,
        row.family(),
        row.first(),
        row.last(),
        &rows,
        weeks,
    ))
}
pub(super) fn daily<S: Snapshot>(
    training: &[S],
    later: &[S],
    facts: &Facts,
) -> Result<Vec<DailyRecord>, String> {
    let mut records = Vec::new();
    records.try_reserve_exact(facts.count).map_err(display)?;
    for (index, (before, after)) in training.iter().zip(later).enumerate() {
        let mut days = sessions(before.periods())?;
        let training_session_count = days.len();
        days.try_reserve_exact(after.periods().len())
            .map_err(display)?;
        days.extend(after.periods().iter().map(|p| Session {
            day: p.day,
            pessimistic_paisa: p.pessimistic_paisa,
            optimistic_paisa: p.optimistic_paisa,
            trades: p.trades,
        }));
        let original = days
            .get(..training_session_count)
            .ok_or("single-stop training day prefix missing")?;
        let later_days = days
            .get(training_session_count..)
            .ok_or("single-stop later day suffix missing")?;
        records.push(DailyRecord {
            binding: daily::Binding {
                original: before.run(),
                later: after.run(),
                later_identity: facts.later.identity,
                later_completion: facts.later.completion,
                source: after.source(),
                family: 0,
                coordinate: u64::try_from(index).map_err(display)?,
            },
            training: index_consistency::evaluate(
                Policy::V1,
                before.family(),
                before.first(),
                before.last(),
                original,
                true,
            ),
            later: index_consistency::evaluate(
                Policy::V1,
                after.family(),
                after.first(),
                after.last(),
                later_days,
                true,
            ),
            evaluation: index_consistency::evaluate(
                Policy::V1,
                before.family(),
                before.first(),
                after.last(),
                &days,
                true,
            ),
            training_session_count,
            sessions: days,
        });
    }
    Ok(records)
}
