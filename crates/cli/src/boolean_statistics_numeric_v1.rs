//! Shared numeric kernels with complete Boolean rows, never selected-only input.
use brutex_core::blake3::Hasher;
use runner::bootstrap::{
    romano_wolf_adjusted_p_values_v1, spa_receipt_v1, white_reality_check_receipt_v1,
};

use crate::population_observations_v1::{CscvLayoutReceiptV1, canonical_masks};
use crate::population_statistics_v2::{
    PopulationStatisticsProcedureV2, cscv_placement, wilson_lower_bits,
};

use super::{
    BooleanCoordinateV1, CandidateStatisticsV1, Group, MeasurementsV1, RomanoWolfAvailabilityV1,
    SplitStatisticsV1, display,
};

pub(super) fn measure(
    group: &Group,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<MeasurementsV1, String> {
    group.require_current()?;
    let mut returns = Vec::new();
    let mut candidates = Vec::new();
    returns
        .try_reserve_exact(group.candidates)
        .map_err(display)?;
    candidates
        .try_reserve_exact(group.candidates)
        .map_err(display)?;
    for (family, source) in group.sources.iter().enumerate() {
        for (coordinate, row) in source.rows().iter().enumerate() {
            let (values, measured) = project_row(family, coordinate, source.sessions(), row)?;
            returns.push(values);
            candidates.push(measured);
        }
    }
    let result = measure_returns(&returns, candidates, group.layout, procedure)?;
    group.require_current()?;
    Ok(result)
}

pub(super) fn project_row(
    family: usize,
    coordinate: usize,
    sessions: &[i64],
    row: &BooleanCoordinateV1,
) -> Result<(Vec<i64>, CandidateStatisticsV1), String> {
    if row.periods().len() != sessions.len() {
        return Err("Boolean statistics coordinate has incomplete sessions".to_owned());
    }
    let mut returns = Vec::new();
    returns.try_reserve_exact(sessions.len()).map_err(display)?;
    let mut total = (0_i64, 0_u64, 0_u64);
    let mut digest = Hasher::new();
    digest.update(b"brutex-boolean-statistics-ordered-periods-v1\0");
    digest.update(&row.identity());
    for (period, &day) in row.periods().iter().zip(sessions) {
        if period.day() != day
            || period.wins() > period.trades()
            || (period.trades() == 0 && period.return_paisa() != 0)
        {
            return Err(
                "Boolean statistics coordinate session/count evidence disagrees".to_owned(),
            );
        }
        total.0 = total
            .0
            .checked_add(period.return_paisa())
            .ok_or("Boolean statistics paisa overflow")?;
        total.1 = total
            .1
            .checked_add(period.trades())
            .ok_or("Boolean statistics trade overflow")?;
        total.2 = total
            .2
            .checked_add(period.wins())
            .ok_or("Boolean statistics win overflow")?;
        returns.push(period.return_paisa());
        digest.update(&day.to_le_bytes());
        digest.update(&period.return_paisa().to_le_bytes());
        digest.update(&period.trades().to_le_bytes());
        digest.update(&period.wins().to_le_bytes());
    }
    if total != (row.cell().pessimistic, row.cell().trades, row.cell().wins) {
        return Err("Boolean statistics sessions do not conserve priced coordinate".to_owned());
    }
    Ok((
        returns,
        CandidateStatisticsV1 {
            family,
            coordinate,
            identity: row.identity(),
            trades: total.1,
            wins: total.2,
            return_paisa: total.0,
            wilson_lower_bits: wilson_lower_bits(total.2, total.1),
            period_digest: digest.finalize(),
        },
    ))
}

pub(super) fn measure_returns(
    returns: &[Vec<i64>],
    candidates: Vec<CandidateStatisticsV1>,
    layout: CscvLayoutReceiptV1,
    procedure: PopulationStatisticsProcedureV2,
) -> Result<MeasurementsV1, String> {
    if returns.len() != candidates.len()
        || returns.is_empty()
        || returns
            .iter()
            .any(|row| row.len() as u64 != layout.period_count())
    {
        return Err("Boolean statistics numeric family is empty or misaligned".to_owned());
    }
    let draws = usize::try_from(procedure.draws()).map_err(display)?;
    let block = usize::try_from(procedure.block_length()).map_err(display)?;
    let white = white_reality_check_receipt_v1(returns, draws, procedure.seed(), block)
        .ok_or("Boolean statistics White procedure refused")?;
    let spa = spa_receipt_v1(returns, draws, procedure.seed(), block)
        .ok_or("Boolean statistics SPA procedure refused")?;
    let romano = romano_wolf_adjusted_p_values_v1(returns, draws, procedure.seed(), block);
    let romano_availability = if romano.is_some() {
        RomanoWolfAvailabilityV1::Measured
    } else if returns
        .iter()
        .any(|row| row.windows(2).all(|pair| pair.first() == pair.get(1)))
    {
        RomanoWolfAvailabilityV1::ConstantReturnCandidate
    } else {
        RomanoWolfAvailabilityV1::NumericalRefusal
    };
    let splits = split_statistics(returns, layout)?;
    let contributing_splits = splits.iter().filter(|row| row.rankable).count() as u64;
    let bottom_half_splits = splits
        .iter()
        .filter(|row| row.rankable && row.bottom_half)
        .count() as u64;
    Ok(MeasurementsV1 {
        white,
        spa,
        romano,
        romano_availability,
        candidates,
        contributing_splits,
        bottom_half_splits,
        splits,
    })
}

fn split_statistics(
    returns: &[Vec<i64>],
    layout: CscvLayoutReceiptV1,
) -> Result<Vec<SplitStatisticsV1>, String> {
    let masks = canonical_masks(layout)?;
    let mut output = Vec::new();
    let mut train = Vec::new();
    let mut test = Vec::new();
    output.try_reserve_exact(masks.len()).map_err(display)?;
    train.try_reserve_exact(returns.len()).map_err(display)?;
    test.try_reserve_exact(returns.len()).map_err(display)?;
    let width = usize::try_from(layout.periods_per_segment()).map_err(display)?;
    for (train_mask, test_mask) in masks {
        train.clear();
        test.clear();
        let mut digest = Hasher::new();
        digest.update(b"brutex-boolean-statistics-split-scores-v1\0");
        digest.update(&layout.digest());
        digest.update(&train_mask.to_le_bytes());
        digest.update(&test_mask.to_le_bytes());
        for row in returns {
            let (train_score, test_score) = split_scores(row, width, train_mask, test_mask)?;
            train.push(train_score);
            test.push(test_score);
            digest.update(&train_score.to_le_bytes());
            digest.update(&test_score.to_le_bytes());
        }
        let (bottom_half, rankable) = cscv_placement(&train, &test)?;
        output.push(SplitStatisticsV1 {
            train_mask,
            test_mask,
            bottom_half,
            rankable,
            scores_digest: digest.finalize(),
        });
    }
    Ok(output)
}

fn split_scores(row: &[i64], width: usize, train: u64, test: u64) -> Result<(i64, i64), String> {
    let mut result = (0_i64, 0_i64);
    for (period, &value) in row.iter().enumerate() {
        let segment = period
            .checked_div(width)
            .ok_or("Boolean CSCV zero segment width")?;
        let bit = 1_u64
            .checked_shl(u32::try_from(segment).map_err(display)?)
            .ok_or("Boolean CSCV segment shift overflow")?;
        if train & bit != 0 && test & bit == 0 {
            result.0 = result
                .0
                .checked_add(value)
                .ok_or("Boolean CSCV train paisa overflow")?;
        } else if test & bit != 0 && train & bit == 0 {
            result.1 = result
                .1
                .checked_add(value)
                .ok_or("Boolean CSCV test paisa overflow")?;
        } else {
            return Err("Boolean CSCV masks do not partition the actual period".to_owned());
        }
    }
    Ok(result)
}
