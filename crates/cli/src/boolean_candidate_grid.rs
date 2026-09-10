//! Frozen actual policy/level metadata for self-contained program drill-down.
use super::reader::Decode;
use super::{ResearchResolvedExitGridV1, display};
use runner::excursion::Side;
use runner::exit_grid_policy::{
    ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1,
    RatioLimitsV1, RationalPercentileV1, RungPlanV1,
};

/// Exact saved source resolution. Reading it grants no execution capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridContext {
    /// Full program execution-grid resolution identity.
    pub resolution: [u8; 32],
    /// Exact TRAINING one-minute bytes identity.
    pub execution: [u8; 32],
    /// Named source feed hash.
    pub feed: [u8; 32],
    /// Exact build identity hash.
    pub commit: [u8; 32],
    /// Canonical execution calendar policy.
    pub calendar: [u8; 32],
    /// Actual TRAINING record count.
    pub bars: u64,
    /// First actual TRAINING timestamp.
    pub first_micros: i64,
    /// Last actual TRAINING timestamp.
    pub last_micros: i64,
    /// Actual complete ratio-admitted coordinate count.
    pub cells: u64,
    /// Explicit horizon in actual one-minute execution bars.
    pub horizon_bars: u32,
    /// Complete explicitly configured execution policy.
    pub policy: ExitGridPolicyV1,
    /// Exact resolved stop distances in parts per million.
    pub stops: Vec<i64>,
    /// Exact resolved target distances in parts per million.
    pub targets: Vec<i64>,
    /// Exact resolved trail distances in parts per million.
    pub trails: Vec<i64>,
}

pub(super) fn encode(
    grids: &[ResearchResolvedExitGridV1; 2],
    horizon: runner::outcome::Horizon,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.try_reserve_exact(usize::try_from(size(grids)?).map_err(display)?)
        .map_err(display)?;
    for grid in grids {
        for digest in [
            grid.digest(),
            grid.training_digest(),
            grid.feed_digest(),
            grid.commit_digest(),
            grid.calendar_digest(),
        ] {
            out.extend_from_slice(&digest);
        }
        word(&mut out, grid.training_bars());
        out.extend_from_slice(&grid.training_first_ts_micros().to_le_bytes());
        out.extend_from_slice(&grid.training_last_ts_micros().to_le_bytes());
        word(&mut out, grid.cell_count());
        word(&mut out, u64::from(horizon.as_bars()));
        policy(&mut out, grid.policy());
        for levels in [
            grid.stop_levels_ppm(),
            grid.target_levels_ppm(),
            grid.trail_levels_ppm(),
        ] {
            word(&mut out, levels.len() as u64);
            for level in levels {
                out.extend_from_slice(&level.to_le_bytes());
            }
        }
    }
    Ok(out)
}
pub(super) fn size(grids: &[ResearchResolvedExitGridV1; 2]) -> Result<u64, String> {
    let mut bytes = 0_u64;
    for grid in grids {
        bytes = bytes
            .checked_add(384)
            .ok_or("Boolean grid metadata size overflow")?;
        for schedule in [
            grid.policy().rungs().stop(),
            grid.policy().rungs().target(),
            grid.policy().rungs().trail(),
        ] {
            bytes = bytes
                .checked_add(
                    (schedule.len() as u64)
                        .checked_mul(16)
                        .ok_or("Boolean policy size overflow")?,
                )
                .ok_or("Boolean grid policy overflow")?;
        }
        for levels in [
            grid.stop_levels_ppm(),
            grid.target_levels_ppm(),
            grid.trail_levels_ppm(),
        ] {
            bytes = bytes
                .checked_add(
                    (levels.len() as u64)
                        .checked_mul(8)
                        .ok_or("Boolean level size overflow")?,
                )
                .ok_or("Boolean grid levels overflow")?;
        }
    }
    Ok(bytes)
}
fn word(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn policy(out: &mut Vec<u8>, policy: &ExitGridPolicyV1) {
    word(
        out,
        match policy.execution_resolution() {
            ExecutionResolutionV1::OneMinuteOhlcv => 60,
            ExecutionResolutionV1::UnsupportedSeconds(seconds) => u64::from(seconds),
        },
    );
    word(
        out,
        match policy.range_resolution() {
            RangeResolutionV1::PpmFloor => 0,
            RangeResolutionV1::PpmCeiling => 1,
        },
    );
    word(
        out,
        match policy.side() {
            Side::Long => 0,
            Side::Short => 1,
        },
    );
    for values in [
        policy.rungs().stop(),
        policy.rungs().target(),
        policy.rungs().trail(),
    ] {
        word(out, values.len() as u64);
        for value in values {
            word(out, u64::from(value.numerator()));
            word(out, u64::from(value.denominator()));
        }
    }
    word(out, policy.rungs().max_levels_per_axis() as u64);
    out.extend_from_slice(&policy.ratios().min_hundredths().to_le_bytes());
    out.extend_from_slice(&policy.ratios().max_hundredths().to_le_bytes());
    word(out, policy.ratios().max_pairs());
    word(out, policy.max_cells());
    word(
        out,
        match policy.selector() {
            ExitGridSelectorV1::PessimisticTotal => 0,
            ExitGridSelectorV1::EdgeThenPessimistic => 1,
            ExitGridSelectorV1::GuaranteedFloor => 2,
        },
    );
    out.extend_from_slice(&policy.cost_model_id());
    let (kind, level) = match policy.forced_stop() {
        ForcedStopV1::Disabled => (0, 0),
        ForcedStopV1::IncludeExactObserved(ppm) => (1, ppm),
        ForcedStopV1::RequireExactObserved(ppm) => (2, ppm),
    };
    word(out, kind);
    out.extend_from_slice(&level.to_le_bytes());
    word(out, policy.max_ambiguous_bars());
    word(out, policy.max_gap_fills());
}

pub(super) fn decode(input: &mut Decode<'_>) -> Result<[GridContext; 2], String> {
    Ok([grid(input, Side::Long)?, grid(input, Side::Short)?])
}
fn grid(input: &mut Decode<'_>, expected_side: Side) -> Result<GridContext, String> {
    let resolution = input.bytes()?;
    let execution = input.bytes()?;
    let feed = input.bytes()?;
    let commit = input.bytes()?;
    let calendar = input.bytes()?;
    let bars = input.word()?;
    let first_micros = input.signed()?;
    let last_micros = input.signed()?;
    let cells = input.word()?;
    let horizon_bars = u32::try_from(input.word()?).map_err(display)?;
    let policy = decode_policy(input)?;
    if policy.side() != expected_side
        || bars == 0
        || cells == 0
        || cells > policy.max_cells()
        || horizon_bars == 0
        || first_micros > last_micros
    {
        return Err("Boolean frozen grid source/cardinality invalid".to_owned());
    }
    Ok(GridContext {
        resolution,
        execution,
        feed,
        commit,
        calendar,
        bars,
        first_micros,
        last_micros,
        cells,
        horizon_bars,
        policy,
        stops: levels(input)?,
        targets: levels(input)?,
        trails: levels(input)?,
    })
}
fn levels(input: &mut Decode<'_>) -> Result<Vec<i64>, String> {
    let count = input.size()?;
    input.admit(count, 8)?;
    let mut levels = Vec::new();
    levels.try_reserve_exact(count).map_err(display)?;
    for _ in 0..count {
        let level = input.signed()?;
        if level <= 0 || levels.last().is_some_and(|previous| *previous >= level) {
            return Err("Boolean saved levels are not positive increasing".to_owned());
        }
        levels.push(level);
    }
    if levels.is_empty() {
        return Err("Boolean saved level axis is empty".to_owned());
    }
    Ok(levels)
}
fn percentiles(input: &mut Decode<'_>) -> Result<Vec<RationalPercentileV1>, String> {
    let count = input.size()?;
    input.admit(count, 16)?;
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(display)?;
    for _ in 0..count {
        values.push(
            RationalPercentileV1::new(
                u32::try_from(input.word()?).map_err(display)?,
                u32::try_from(input.word()?).map_err(display)?,
            )
            .map_err(display)?,
        );
    }
    Ok(values)
}
fn decode_policy(input: &mut Decode<'_>) -> Result<ExitGridPolicyV1, String> {
    if input.word()? != 60 {
        return Err("Boolean saved grid execution is not one minute".to_owned());
    }
    let range = match input.word()? {
        0 => RangeResolutionV1::PpmFloor,
        1 => RangeResolutionV1::PpmCeiling,
        _ => return Err("Boolean range encoding unknown".to_owned()),
    };
    let side = match input.word()? {
        0 => Side::Long,
        1 => Side::Short,
        _ => return Err("Boolean policy side unknown".to_owned()),
    };
    let stop = percentiles(input)?;
    let target = percentiles(input)?;
    let trail = percentiles(input)?;
    let rungs = RungPlanV1::new(stop, target, trail, input.size()?).map_err(display)?;
    let ratios =
        RatioLimitsV1::new(input.signed()?, input.signed()?, input.word()?).map_err(display)?;
    let cells = input.word()?;
    let selector = match input.word()? {
        0 => ExitGridSelectorV1::PessimisticTotal,
        1 => ExitGridSelectorV1::EdgeThenPessimistic,
        2 => ExitGridSelectorV1::GuaranteedFloor,
        _ => return Err("Boolean selector unknown".to_owned()),
    };
    let cost = input.bytes()?;
    let forced_kind = input.word()?;
    let level = input.signed()?;
    let forced = match (forced_kind, level) {
        (0, 0) => ForcedStopV1::Disabled,
        (1, level) => ForcedStopV1::IncludeExactObserved(level),
        (2, level) => ForcedStopV1::RequireExactObserved(level),
        _ => return Err("Boolean forced-stop encoding unknown".to_owned()),
    };
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        range,
        side,
        rungs,
        ratios,
        cells,
        selector,
        cost,
        forced,
        input.word()?,
        input.word()?,
    )
    .map_err(display)
}
