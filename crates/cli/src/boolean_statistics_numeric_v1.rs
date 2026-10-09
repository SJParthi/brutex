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
    // EACH ROW'S PERIODS ARE WALKED ONCE, NOT ONCE PER SPLIT (p2bool-2,
    // D-2639). The summaries hold at most one entry per segment (CO-02 caps a
    // layout at 16), and each carries its running-sum extremes, so a split
    // refuses on exactly the inputs and with exactly the sentence the
    // per-period walk did: proven against that walk, kept as the test oracle,
    // by `cli::candidate_universe::boolean_candidate_v1::statistics::numeric::tests::segment_sums_match_the_period_walk`.
    let mut summaries = Vec::new();
    if !masks.is_empty() {
        summaries
            .try_reserve_exact(returns.len())
            .map_err(display)?;
        for row in returns {
            summaries.push(segment_sums(row, width)?);
        }
    }
    for (train_mask, test_mask) in masks {
        train.clear();
        test.clear();
        let mut digest = Hasher::new();
        digest.update(b"brutex-boolean-statistics-split-scores-v1\0");
        digest.update(&layout.digest());
        digest.update(&train_mask.to_le_bytes());
        digest.update(&test_mask.to_le_bytes());
        for row in &summaries {
            let (train_score, test_score) = split_scores_of(row, train_mask, test_mask)?;
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

/// One segment of one row: its exact sum and the highest and lowest running
/// sum reached inside it, exact in `i128` (p2bool-2, D-2639).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    total: i128,
    high: i128,
    low: i128,
}

/// The row's segment summaries, in segment order; segment `k` is entry `k`.
fn segment_sums(row: &[i64], width: usize) -> Result<Vec<Segment>, String> {
    let mut out: Vec<Segment> = Vec::new();
    for (period, &value) in row.iter().enumerate() {
        let segment = period
            .checked_div(width)
            .ok_or("Boolean CSCV zero segment width")?;
        let value = i128::from(value);
        if let Some(open) = out.get_mut(segment) {
            open.total = open
                .total
                .checked_add(value)
                .ok_or("Boolean CSCV segment sum overflow")?;
            open.high = open.high.max(open.total);
            open.low = open.low.min(open.total);
        } else {
            out.try_reserve(1).map_err(display)?;
            out.push(Segment {
                total: value,
                high: value,
                low: value,
            });
        }
    }
    Ok(out)
}

/// `running` plus one segment, or `None` when a running sum the per-period
/// walk reached inside it leaves `i64`.
fn add_segment(running: i128, segment: Segment) -> Option<i128> {
    let high = running.checked_add(segment.high)?;
    let low = running.checked_add(segment.low)?;
    if high > i128::from(i64::MAX) || low < i128::from(i64::MIN) {
        return None;
    }
    running.checked_add(segment.total)
}

/// One split's `(train, test)` from a row's summaries, refusing with the
/// per-period walk's sentences in its order.
fn split_scores_of(segments: &[Segment], train: u64, test: u64) -> Result<(i64, i64), String> {
    let mut result = (0_i128, 0_i128);
    for (segment, summary) in segments.iter().copied().enumerate() {
        let bit = 1_u64
            .checked_shl(u32::try_from(segment).map_err(display)?)
            .ok_or("Boolean CSCV segment shift overflow")?;
        if train & bit != 0 && test & bit == 0 {
            result.0 = add_segment(result.0, summary).ok_or("Boolean CSCV train paisa overflow")?;
        } else if test & bit != 0 && train & bit == 0 {
            result.1 = add_segment(result.1, summary).ok_or("Boolean CSCV test paisa overflow")?;
        } else {
            return Err("Boolean CSCV masks do not partition the actual period".to_owned());
        }
    }
    Ok((
        i64::try_from(result.0).map_err(display)?,
        i64::try_from(result.1).map_err(display)?,
    ))
}

/// The per-period walk `split_statistics` ran before p2bool-2, kept as the
/// oracle the segment summaries are proven against.
#[cfg(test)]
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

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "a five-entry constant table indexed by a remainder of five"
)]
mod tests {
    use super::{segment_sums, split_scores, split_scores_of};

    /// p2bool-2 (D-2639): the segment summaries give the per-period walk's
    /// `(train, test)` and its refusal sentence on every input, exhaustively
    /// over every row of up to five periods from {MIN, -1, 0, 1, MAX}, widths
    /// 0 to 3, every 6-bit train mask against its complement, the full mask,
    /// the empty mask and itself (an overlap); plus 70 one-period segments,
    /// which reach a shift past 63.
    #[test]
    fn segment_sums_match_the_period_walk() {
        const VALUES: [i64; 5] = [i64::MIN, -1, 0, 1, i64::MAX];
        let mut compared = 0_u64;
        for len in 0..=5_u32 {
            for code in 0..5_usize.pow(len) {
                let mut rest = code;
                let mut row = Vec::new();
                for _ in 0..len {
                    row.push(VALUES[rest % 5]);
                    rest /= 5;
                }
                for width in 0..=3_usize {
                    let summaries = segment_sums(&row, width);
                    for train in 0..64_u64 {
                        for test in [!train & 0x3f, 0x3f, 0, train] {
                            let expected = split_scores(&row, width, train, test);
                            let got = summaries
                                .clone()
                                .and_then(|s| split_scores_of(&s, train, test));
                            assert_eq!(
                                got, expected,
                                "row {row:?} width {width} masks {train:#x}/{test:#x}"
                            );
                            compared += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(compared, (1 + 5 + 25 + 125 + 625 + 3125) * 4 * 64 * 4);
        let wide = vec![1_i64; 70];
        for (train, test) in [(u64::MAX, 0), (0, u64::MAX), (u64::MAX >> 1, 1 << 63)] {
            assert_eq!(
                segment_sums(&wide, 1).and_then(|s| split_scores_of(&s, train, test)),
                split_scores(&wide, 1, train, test)
            );
        }
        assert_eq!(
            split_scores(&wide, 1, u64::MAX, 0),
            Err("Boolean CSCV segment shift overflow".to_owned())
        );
    }
}
