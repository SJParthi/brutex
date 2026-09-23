//! Later-period observations under an opaque complete TRAINING program/grid.
//! No constructor re-resolves exits from later prices or mints admission authority.
#[path = "expression_validation.rs"]
pub mod validation;

use super::{ExpressionExecutionRunV1, ResearchResolvedExitGridV1, ValidatedExpressionGridV1};
use crate::{
    expression::{Expression, Summary, Truth},
    grid::{Grid, TradeRow},
    identity::RunId,
    outcome::Horizon,
};
use indicators::{
    Candle,
    column::{Column, EvaluationSpecToken},
};

/// Immutable provenance of a complete authenticated training grid, including
/// programs with no hits. It cannot be constructed from saved observer bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionTrainingAnchorV1 {
    program: Expression,
    run: RunId,
    resolution: [u8; 32],
    evaluation: [u8; 32],
    column: [u8; 32],
    spec: EvaluationSpecToken,
    horizon: Horizon,
    digest: [u8; 32],
}
impl ValidatedExpressionGridV1<'_> {
    /// Preserve the full training program and already validated grid authority.
    #[must_use]
    pub fn later_period_anchor(&self) -> ExpressionTrainingAnchorV1 {
        let e = self.evaluated;
        let mut anchor = ExpressionTrainingAnchorV1 {
            program: e.program().clone(),
            run: e.run_id(),
            resolution: e.resolution,
            evaluation: self.digest,
            column: e.column_digest,
            spec: e.evaluation_spec,
            horizon: e.horizon,
            digest: [0; 32],
        };
        anchor.digest = anchor.seal();
        anchor
    }
}
impl ExpressionTrainingAnchorV1 {
    /// Complete original executable program.
    #[must_use]
    pub const fn program(&self) -> &Expression {
        &self.program
    }
    /// Exact directional training run.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run
    }
    /// Frozen training resolution identity.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.resolution
    }
    /// Complete training evidence identity.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Original execution horizon; later callers cannot replace it.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }
    fn seal(&self) -> [u8; 32] {
        let mut h = super::super::Hasher::new();
        h.update(b"brutex.runner.expression-training-anchor.v1\0");
        for value in [
            self.run.bytes(),
            self.resolution,
            self.evaluation,
            self.column,
        ] {
            h.update(&value);
        }
        h.update(&self.program.encode());
        h.update(&self.horizon.as_bars().to_le_bytes());
        h.update(self.spec.fingerprint_v1().as_bytes());
        h.finalize()
    }
}

/// Complete later-period observations using only the frozen TRAINING levels.
/// Borrowed bars/column remain immutable through every exact trade materialization.
pub struct EvaluatedExpressionOosV1<'a> {
    bars: &'a [Candle],
    column: &'a Column,
    anchor: ExpressionTrainingAnchorV1,
    run: ExpressionExecutionRunV1,
    grid: Grid,
    side: crate::excursion::Side,
    summary: Summary,
    support_sessions: u64,
    digest: [u8; 32],
    refusals: Vec<super::super::ExecutionRefusalBitsV1>,
}
impl EvaluatedExpressionOosV1<'_> {
    /// Identity of the complete later program, source, frozen grid and measured cells.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Actual later-period run, distinct from the training run.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run.run_id()
    }
    /// Every coordinate, including zero-trade and execution-policy-refused cells.
    #[must_use]
    pub const fn grid(&self) -> &Grid {
        &self.grid
    }
    /// Definite and unavailable later signal counts.
    #[must_use]
    pub const fn summary(&self) -> Summary {
        self.summary
    }
    /// Actual later sessions with a definitely true signal.
    #[must_use]
    pub const fn support_sessions(&self) -> u64 {
        self.support_sessions
    }
    /// Existing execution-policy comparison on this later measured coordinate.
    #[must_use]
    pub fn refusal_bits(&self, ordinal: usize) -> Option<super::super::ExecutionRefusalBitsV1> {
        self.refusals.get(ordinal).copied()
    }
    /// Materialize exactly one frozen coordinate, without choosing a winner.
    /// # Errors
    /// Refuses absent ordinals or any mismatch between the grid and actual trades.
    pub fn materialize(&self, ordinal: usize) -> Result<Vec<TradeRow>, String> {
        let cell = self
            .grid
            .cells
            .get(ordinal)
            .ok_or("later expression coordinate absent")?;
        crate::grid::materialize_expression_cell(
            self.bars,
            self.column,
            self.anchor.program(),
            self.anchor.horizon,
            self.side,
            &self.grid,
            cell,
        )
    }
}

impl ResearchResolvedExitGridV1 {
    /// Replay all original coordinates on strictly later actual one-minute bars.
    /// Source terms, evaluator, program and horizon must match the training anchor;
    /// no percentile, ratio, stop or trail is recomputed from later prices.
    /// # Errors
    /// Refuses earlier/overlapping sessions, changed provenance/evaluator/program,
    /// corrupt or incomplete execution acceptance and any partial grid result.
    pub fn evaluate_expression_oos<'a>(
        &self,
        anchor: &ExpressionTrainingAnchorV1,
        series: super::super::ExecutionSeriesV1<'a>,
        column: &'a Column,
        run: &ExpressionExecutionRunV1,
    ) -> Result<EvaluatedExpressionOosV1<'a>, String> {
        let view = self.view();
        view.require_runtime_integrity().map_err(super::display)?;
        view.require_matching_series(series)
            .map_err(super::display)?;
        if anchor.digest != anchor.seal()
            || anchor.resolution != self.digest()
            || run.program() != anchor.program()
            || column.evaluation_spec_token() != Some(anchor.spec)
        {
            return Err(
                "later expression training/program/resolution/evaluator authority differs".into(),
            );
        }
        let bars = series.bars();
        super::super::validate_execution_bars(bars).map_err(super::display)?;
        let first = bars
            .first()
            .ok_or("later expression execution is empty")?
            .ts_micros;
        let day = |ts: i64| (i128::from(ts) + 19_800_000_000).div_euclid(86_400_000_000);
        if first <= self.training_last_ts_micros()
            || day(first) <= day(self.training_last_ts_micros())
        {
            return Err("later expression requires a strictly later actual IST session".into());
        }
        super::super::require_complete_acceptance(column, bars.len()).map_err(super::display)?;
        super::super::validate_column_sources(column, bars.len(), 0).map_err(super::display)?;
        if column.is_empty() {
            return Err("later expression has no warm reachable signal rows".into());
        }
        super::super::validate_arithmetic_envelope_view(bars, &view).map_err(super::display)?;
        let execution = crate::identity::data_digest(bars);
        run.source
            .require_matches_terms(
                &view.instrument,
                view.feed_digest,
                view.commit_digest,
                execution,
                view.side(),
            )
            .map_err(super::display)?;
        let grid = crate::grid::evaluate_resolved_expression_policy_v1(
            bars,
            column,
            anchor.program(),
            anchor.horizon,
            view.side(),
            &view,
        )?;
        view.validate_complete_grid(&grid).map_err(super::display)?;
        let (summary, support_sessions) = summarize(column, bars, anchor.program())?;
        if summary.hits != grid.signals {
            return Err("later expression truth/grid signals differ".into());
        }
        let digest = seal(
            anchor,
            run,
            column,
            execution,
            &grid,
            summary,
            support_sessions,
        );
        let mut refusals = Vec::new();
        refusals
            .try_reserve_exact(grid.cells.len())
            .map_err(super::display)?;
        refusals.extend(
            grid.cells
                .iter()
                .map(|cell| view.execution_refusal_bits(cell)),
        );
        Ok(EvaluatedExpressionOosV1 {
            bars,
            column,
            anchor: anchor.clone(),
            run: run.clone(),
            grid,
            side: view.side(),
            summary,
            support_sessions,
            digest,
            refusals,
        })
    }
}
fn summarize(
    column: &Column,
    bars: &[Candle],
    program: &Expression,
) -> Result<(Summary, u64), String> {
    let mut support = 0_u64;
    let mut last = None;
    let summary = crate::expression::evaluate(column, program, |index, truth| {
        if truth == Truth::True {
            let stamp = bars
                .get(index)
                .ok_or("later expression support source absent")?
                .ts_micros;
            let day = (i128::from(stamp) + 19_800_000_000).div_euclid(86_400_000_000);
            if last != Some(day) {
                support = support
                    .checked_add(1)
                    .ok_or("later support count overflow")?;
                last = Some(day);
            }
        }
        Ok::<(), &'static str>(())
    })
    .map_err(|why| format!("later expression summary refused: {why:?}"))?;
    Ok((summary, support))
}
fn seal(
    anchor: &ExpressionTrainingAnchorV1,
    run: &ExpressionExecutionRunV1,
    column: &Column,
    execution: [u8; 32],
    grid: &Grid,
    summary: Summary,
    support: u64,
) -> [u8; 32] {
    let mut h = super::super::Hasher::new();
    h.update(b"brutex.runner.expression-later-period.v1\0");
    for value in [
        anchor.digest(),
        run.run_id().bytes(),
        execution,
        super::super::digest_column(column),
    ] {
        h.update(&value);
    }
    for value in [
        summary.evaluated,
        summary.hits,
        summary.misses,
        summary.unknown,
        support,
        grid.cells.len() as u64,
    ] {
        h.update(&value.to_le_bytes());
    }
    for cell in &grid.cells {
        super::super::put_cell(&mut h, cell);
    }
    h.finalize()
}

#[cfg(test)]
#[path = "expression_oos_tests.rs"]
mod tests;
