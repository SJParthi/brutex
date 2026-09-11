//! Borrowed common geometry from a sealed legacy or research resolution.
use super::research_resolution::ResearchResolvedExitGridV1;
use super::{
    AttestedTrainingV1, ExecutionRefusalBitsV1, ExecutionSeriesV1, ExitGridErrorV1,
    ExitGridPolicyV1, ForcedStopV1, ResolvedExitGridV1, ResolvedLaddersV1, checked_sum_below,
    coordinate_row_width, digest_column, hash, require_complete_acceptance,
    validate_arithmetic_envelope_view, validate_column_sources, validate_execution_bars,
};
use crate::{
    excursion::{Ladder, Ppm, Side},
    grid::{Cell, Chosen, Grid},
    outcome::Horizon,
};
use brutex_core::instrument::InstrumentKey;

enum Owner<'a> {
    Legacy(&'a ResolvedExitGridV1),
    Research(&'a ResearchResolvedExitGridV1),
}

pub(crate) struct ResolvedGridViewV1<'a> {
    owner: Owner<'a>,
    pub(crate) instrument: InstrumentKey,
    pub(crate) feed_digest: [u8; 32],
    pub(crate) commit_digest: [u8; 32],
    pub(crate) calendar_digest: [u8; 32],
    pub(crate) training_digest: [u8; 32],
    pub(crate) training_bars: u64,
    pub(crate) digest: [u8; 32],
    pub(crate) policy: &'a ExitGridPolicyV1,
    pub(crate) stop_levels_ppm: &'a [Ppm],
    pub(crate) target_levels_ppm: &'a [Ppm],
    pub(crate) trail_levels_ppm: &'a [Ppm],
    pub(crate) ratio_bitmap: &'a [bool],
    pub(crate) forced_stop_index: Option<usize>,
    pub(crate) cell_count: u64,
}

impl ResolvedExitGridV1 {
    pub(crate) fn view(&self) -> ResolvedGridViewV1<'_> {
        ResolvedGridViewV1 {
            owner: Owner::Legacy(self),
            instrument: self.instrument,
            feed_digest: self.feed_digest,
            commit_digest: self.commit_digest,
            calendar_digest: self.calendar_digest,
            training_digest: self.training_digest,
            training_bars: self.training_bars,
            digest: self.digest,
            policy: &self.policy,
            stop_levels_ppm: &self.stop_levels_ppm,
            target_levels_ppm: &self.target_levels_ppm,
            trail_levels_ppm: &self.trail_levels_ppm,
            ratio_bitmap: self.ratio_bitmap(),
            forced_stop_index: self.forced_stop_index,
            cell_count: self.cell_count,
        }
    }
}

impl ResearchResolvedExitGridV1 {
    pub(crate) fn view(&self) -> ResolvedGridViewV1<'_> {
        ResolvedGridViewV1 {
            owner: Owner::Research(self),
            instrument: self.instrument,
            feed_digest: self.feed_digest,
            commit_digest: self.commit_digest,
            calendar_digest: self.calendar_digest,
            training_digest: self.training_digest,
            training_bars: self.training_bars,
            digest: self.digest,
            policy: &self.policy,
            stop_levels_ppm: &self.levels.stops,
            target_levels_ppm: &self.levels.targets,
            trail_levels_ppm: &self.levels.trails,
            ratio_bitmap: &self.levels.ratio_bitmap,
            forced_stop_index: self.levels.forced_stop_index,
            cell_count: self.levels.cell_count,
        }
    }

    /// Attest exact source, column, horizon and full arithmetic envelope once.
    ///
    /// # Errors
    /// The shared legacy source/column/refusal checks apply with this research
    /// resolution's distinct full-key and membership-bound identity.
    pub fn attest_training<'a>(
        &self,
        series: ExecutionSeriesV1<'a>,
        column: &'a indicators::column::Column,
        horizon: Horizon,
    ) -> Result<AttestedTrainingV1<'a>, ExitGridErrorV1> {
        self.view().attest_training(series, column, horizon)
    }
}

impl ResolvedGridViewV1<'_> {
    pub(crate) fn require_runtime_integrity(&self) -> Result<(), ExitGridErrorV1> {
        match self.owner {
            Owner::Legacy(value) => value.require_runtime_integrity(),
            Owner::Research(value) => value.require_runtime_integrity(),
        }
    }
    pub(crate) fn side(&self) -> Side {
        self.policy.side()
    }
    pub(crate) fn stop_levels_ppm(&self) -> &[Ppm] {
        self.stop_levels_ppm
    }
    pub(crate) fn target_levels_ppm(&self) -> &[Ppm] {
        self.target_levels_ppm
    }
    pub(crate) fn trail_levels_ppm(&self) -> &[Ppm] {
        self.trail_levels_ppm
    }
    pub(crate) fn ratio_bitmap(&self) -> &[bool] {
        self.ratio_bitmap
    }
    pub(crate) fn cell_count(&self) -> u64 {
        self.cell_count
    }
    pub(crate) fn ladders(&self) -> Result<ResolvedLaddersV1, ExitGridErrorV1> {
        let ladder = |values: &[Ppm], name| {
            let mut copy = Vec::new();
            copy.try_reserve_exact(values.len()).map_err(|_| {
                ExitGridErrorV1::BufferAllocationRefused {
                    buffer: "exact expression ladder",
                    elements: values.len(),
                }
            })?;
            copy.extend_from_slice(values);
            Ladder::new(copy).ok_or(ExitGridErrorV1::InvalidResolvedLadder(name))
        };
        Ok(ResolvedLaddersV1 {
            stops: ladder(self.stop_levels_ppm, "stop")?,
            targets: ladder(self.target_levels_ppm, "target")?,
            trails: ladder(self.trail_levels_ppm, "trail")?,
        })
    }
}

impl ResolvedGridViewV1<'_> {
    pub(crate) fn attest_training<'a>(
        &self,
        series: ExecutionSeriesV1<'a>,
        column: &'a indicators::column::Column,
        horizon: Horizon,
    ) -> Result<AttestedTrainingV1<'a>, ExitGridErrorV1> {
        self.require_runtime_integrity()?;
        self.require_matching_series(series)?;
        let bars = series.bars();
        validate_execution_bars(bars)?;
        let count = u64::try_from(bars.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("training replay bar count"))?;
        if count != self.training_bars || crate::identity::data_digest(bars) != self.training_digest
        {
            return Err(ExitGridErrorV1::TrainingSeriesMismatch);
        }
        let evaluation_spec = column
            .evaluation_spec_token()
            .ok_or(ExitGridErrorV1::MissingEvaluationSpec)?;
        require_complete_acceptance(column, bars.len())?;
        validate_column_sources(column, bars.len(), 0)?;
        validate_arithmetic_envelope_view(bars, self)?;
        let column_digest = digest_column(column);
        Ok(AttestedTrainingV1 {
            resolution_digest: self.digest,
            bars,
            column,
            horizon,
            column_digest,
            evaluation_spec,
        })
    }
    pub(crate) fn validate_complete_grid(&self, grid: &Grid) -> Result<(), ExitGridErrorV1> {
        if grid.stops.rungs() != self.stop_levels_ppm()
            || grid.targets.rungs() != self.target_levels_ppm()
            || grid.trails.rungs() != self.trail_levels_ppm()
        {
            return Err(ExitGridErrorV1::ReplayLadderMismatch);
        }
        let cells = u64::try_from(grid.cells.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("replay cell count"))?;
        if cells != self.cell_count {
            return Err(ExitGridErrorV1::ReplayCellCountMismatch {
                expected: self.cell_count,
                actual: cells,
            });
        }
        if !self.coordinate_sequence_is_complete(&grid.cells) {
            return Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch);
        }
        if grid.refused_paths != 0 {
            return Err(ExitGridErrorV1::RefusedExecutionPaths {
                paths: grid.refused_paths,
            });
        }
        Ok(())
    }
    pub(crate) fn require_matching_series(
        &self,
        series: ExecutionSeriesV1<'_>,
    ) -> Result<(), ExitGridErrorV1> {
        if *series.instrument() != self.instrument {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("instrument"));
        }
        if hash(series.feed().as_bytes()) != self.feed_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("feed"));
        }
        if hash(series.commit().as_bytes()) != self.commit_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("commit"));
        }
        if series.calendar_digest() != self.calendar_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("calendar"));
        }
        Ok(())
    }
    pub(crate) fn coordinate_row_offsets(&self) -> Result<Vec<Option<usize>>, ExitGridErrorV1> {
        let stop_settings = self.stop_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate stop settings"),
        )?;
        let target_settings = self.target_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate target settings"),
        )?;
        let row_count = stop_settings.checked_mul(target_settings).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate row-offset count"),
        )?;
        let mut offsets = Vec::new();
        offsets.try_reserve_exact(row_count).map_err(|_| {
            ExitGridErrorV1::BufferAllocationRefused {
                buffer: "coordinate row offsets",
                elements: row_count,
            }
        })?;
        let mut ordinal = 0_usize;
        for stop_setting in 0..stop_settings {
            for target_setting in 0..target_settings {
                let stop = (stop_setting < self.stop_levels_ppm.len()).then_some(stop_setting);
                let target =
                    (target_setting < self.target_levels_ppm.len()).then_some(target_setting);
                if let (Some(stop_index), Some(target_index)) = (stop, target)
                    && !self.ratio_pair_admitted(stop_index, target_index)
                {
                    offsets.push(None);
                    continue;
                }
                offsets.push(Some(ordinal));
                ordinal = ordinal
                    .checked_add(coordinate_row_width(
                        target_setting,
                        self.trail_levels_ppm.len(),
                    )?)
                    .ok_or(ExitGridErrorV1::ArithmeticOverflow("coordinate row offset"))?;
            }
        }
        let actual = u64::try_from(ordinal)
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("coordinate population width"))?;
        if actual != self.cell_count {
            return Err(ExitGridErrorV1::ReplayCellCountMismatch {
                expected: self.cell_count,
                actual,
            });
        }
        Ok(offsets)
    }
    pub(crate) fn canonical_coordinate_ordinal(
        &self,
        row_offsets: &[Option<usize>],
        coordinate: Chosen,
    ) -> Result<Option<usize>, ExitGridErrorV1> {
        let stop_setting = coordinate.stop.unwrap_or(self.stop_levels_ppm.len());
        let target_setting = coordinate.target.unwrap_or(self.target_levels_ppm.len());
        let target_settings = self.target_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate target settings"),
        )?;
        let row_slot = stop_setting
            .checked_mul(target_settings)
            .and_then(|row| row.checked_add(target_setting))
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate row-offset slot",
            ))?;
        let Some(row_start) = row_offsets.get(row_slot).copied().flatten() else {
            return Ok(None);
        };
        let trail_setting = coordinate.tsl.unwrap_or(self.trail_levels_ppm.len());
        let prior_trail_groups = trail_setting
            .checked_add(
                target_setting
                    .checked_mul(checked_sum_below(trail_setting)?)
                    .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                        "coordinate prior armed trails",
                    ))?,
            )
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate trail-group offset",
            ))?;
        let within_group = match coordinate.ttp {
            None => 0,
            Some(ttp) => ttp
                .arm
                .checked_mul(trail_setting)
                .and_then(|arm| arm.checked_add(ttp.trail))
                .and_then(|offset| offset.checked_add(1))
                .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                    "coordinate armed-trail offset",
                ))?,
        };
        row_start
            .checked_add(prior_trail_groups)
            .and_then(|offset| offset.checked_add(within_group))
            .map(Some)
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate canonical ordinal",
            ))
    }
    pub(crate) fn coordinate_sequence_is_complete(&self, cells: &[Cell]) -> bool {
        let mut actual = cells.iter();
        let visited = self.visit_coordinates(|expected| {
            actual
                .next()
                .is_some_and(|cell| Chosen::from_cell(cell) == expected)
        });
        visited && actual.next().is_none()
    }
    pub(crate) fn visit_coordinates(&self, mut visit: impl FnMut(Chosen) -> bool) -> bool {
        for stop_setting in 0..=self.stop_levels_ppm.len() {
            for target_setting in 0..=self.target_levels_ppm.len() {
                let stop = (stop_setting < self.stop_levels_ppm.len()).then_some(stop_setting);
                let target =
                    (target_setting < self.target_levels_ppm.len()).then_some(target_setting);
                if let (Some(stop_index), Some(target_index)) = (stop, target)
                    && !self.ratio_pair_admitted(stop_index, target_index)
                {
                    continue;
                }
                for trail_setting in 0..=self.trail_levels_ppm.len() {
                    let tsl =
                        (trail_setting < self.trail_levels_ppm.len()).then_some(trail_setting);
                    let cap = tsl.unwrap_or(self.trail_levels_ppm.len());
                    if !visit(Chosen {
                        stop,
                        target,
                        tsl,
                        ttp: None,
                    }) {
                        return false;
                    }
                    for arm in 0..target_setting {
                        for trail in 0..cap {
                            if !visit(Chosen {
                                stop,
                                target,
                                tsl,
                                ttp: Some(crate::grid::Ttp { arm, trail }),
                            }) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }
    pub(crate) fn ratio_pair_admitted(&self, stop_index: usize, target_index: usize) -> bool {
        stop_index
            .checked_mul(self.target_levels_ppm.len())
            .and_then(|row| row.checked_add(target_index))
            .and_then(|slot| self.ratio_bitmap.get(slot))
            .copied()
            .unwrap_or(false)
    }
    pub(crate) fn chosen_is_in_bounds(&self, chosen: Chosen) -> bool {
        if !self.chosen_axes_are_in_bounds(chosen) {
            return false;
        }
        let (Some(stop_index), Some(target_index)) = (chosen.stop, chosen.target) else {
            // Baseline/one-sided rows remain in the complete comparison grid,
            // but a policy that states reward-to-risk limits may not select or
            // replay a coordinate for which that ratio is undefined.
            return false;
        };
        self.ratio_pair_admitted(stop_index, target_index)
    }
    pub(crate) fn chosen_axes_are_in_bounds(&self, chosen: Chosen) -> bool {
        if chosen.stop.is_some_and(|i| i >= self.stop_levels_ppm.len())
            || chosen
                .target
                .is_some_and(|i| i >= self.target_levels_ppm.len())
            || chosen.tsl.is_some_and(|i| i >= self.trail_levels_ppm.len())
        {
            return false;
        }
        if let Some(ttp) = chosen.ttp
            && (ttp.arm >= self.target_levels_ppm.len()
                || ttp.trail >= self.trail_levels_ppm.len()
                || chosen.target.is_some_and(|target| ttp.arm >= target)
                || chosen.tsl.is_some_and(|tsl| ttp.trail >= tsl))
        {
            return false;
        }
        true
    }
    pub(crate) fn execution_refusal_bits(&self, cell: &Cell) -> ExecutionRefusalBitsV1 {
        let mut bits = ExecutionRefusalBitsV1::NONE;
        if cell.stop.is_none() {
            bits = bits.union(ExecutionRefusalBitsV1::MISSING_STOP);
        }
        if cell.target.is_none() {
            bits = bits.union(ExecutionRefusalBitsV1::MISSING_TARGET);
        }
        if cell.trades == 0 {
            bits = bits.union(ExecutionRefusalBitsV1::ZERO_TRADES);
        }
        if cell.ambiguous_bars > self.policy.max_ambiguous_bars() {
            bits = bits.union(ExecutionRefusalBitsV1::AMBIGUITY_LIMIT);
        }
        if cell.gapped > self.policy.max_gap_fills() {
            bits = bits.union(ExecutionRefusalBitsV1::GAP_LIMIT);
        }
        if matches!(
            self.policy.forced_stop(),
            ForcedStopV1::RequireExactObserved(_)
        ) && cell.stop != self.forced_stop_index
        {
            bits = bits.union(ExecutionRefusalBitsV1::FORCED_STOP_MISMATCH);
        }
        bits
    }
}
