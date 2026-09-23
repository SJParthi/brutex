//! Program-aware execution authority over the complete resolved exit grid.
//!
//! These capabilities have no conversion to the closed-AND V1 capabilities.
//! Full program bytes, source identity, exact policy coordinates and evaluator
//! identity stay bound together. Grid-policy authorization is not institutional
//! statistical admission. Construction/hashing/replay are input-dependent;
//! after validation, one canonical coordinate is a bounded lookup.

use super::research_resolution::ResearchResolvedExitGridV1;
use super::{
    AttestedTrainingV1, ExecutionRefusalBitsV1, ExecutionRunV1, ResolvedExitGridV1,
    ResolvedGridViewV1,
};
use crate::expression::Expression;
use crate::grid::{Cell, Chosen, Grid, TradeRow};
use crate::identity::{DailyReferenceBinding, Run, RunId};
use crate::outcome::Horizon;
use indicators::{Candle, column::EvaluationSpecToken};
use std::sync::Arc;

#[path = "expression_oos.rs"]
pub mod later_period;

/// Exact program and nine-term source identity, sealed from actual source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionExecutionRunV1 {
    source: ExecutionRunV1,
    program: Arc<Expression>,
    identity: RunId,
}

impl ExpressionExecutionRunV1 {
    /// Bind a complete program and the exact signal/execution bytes.
    ///
    /// # Errors
    /// Refuses source identity mismatches or a referenced-bit term that does
    /// not equal this program. The term is metadata, never an AND predicate.
    pub fn new(
        run: &Run<'_>,
        program: &Expression,
        signal: &[Candle],
        execution: Option<&[Candle]>,
    ) -> Result<Self, String> {
        let source = ExecutionRunV1::new(run, signal, execution).map_err(display)?;
        Self::seal(run, program, source)
    }

    /// Bind the full stored daily/minute-reference authority as well as program.
    ///
    /// # Errors
    /// Every ordinary reference-aware execution identity check applies before
    /// the program-aware capability is minted.
    pub fn new_with_daily_reference(
        run: &Run<'_>,
        program: &Expression,
        signal: &[Candle],
        minute_context: &[Candle],
        execution: &[Candle],
        reference: DailyReferenceBinding<'_>,
    ) -> Result<Self, String> {
        let source = ExecutionRunV1::new_with_daily_reference(
            run,
            signal,
            minute_context,
            execution,
            reference,
        )
        .map_err(display)?;
        Self::seal(run, program, source)
    }

    fn seal(run: &Run<'_>, program: &Expression, source: ExecutionRunV1) -> Result<Self, String> {
        if run.mask != program.referenced() {
            return Err(
                "expression run referenced-bit identity differs from its program".to_owned(),
            );
        }
        Ok(Self {
            source,
            program: Arc::new(program.clone()),
            identity: crate::identity::identity_with_expression(run, program),
        })
    }

    /// Canonical nine-term identity including the entire encoded program.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.identity
    }

    /// Actual executable three-valued program.
    #[must_use]
    pub fn program(&self) -> &Expression {
        &self.program
    }

    /// Exact execution bytes bound into the source identity.
    #[must_use]
    pub const fn execution_digest(&self) -> [u8; 32] {
        self.source.execution_digest()
    }
}

/// Complete evaluated coordinate population for one explicit program and side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedExpressionGridV1 {
    resolution: [u8; 32],
    run: ExpressionExecutionRunV1,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec: EvaluationSpecToken,
    side: crate::excursion::Side,
    grid: Grid,
    summary: crate::expression::Summary,
    support_sessions: u64,
}

impl EvaluatedExpressionGridV1 {
    /// Complete program-aware execution identity.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run.run_id()
    }

    /// Program whose truth values selected entry signals.
    #[must_use]
    pub fn program(&self) -> &Expression {
        self.run.program()
    }

    /// Frozen TRAINING resolution identity.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.resolution
    }

    /// Complete unranked coordinate population, including zero-trade cells.
    #[must_use]
    pub const fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Exact execution horizon in minutes.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }

    /// Direction of this coordinate population.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.side
    }

    /// Exact source-coordinate/evaluator column identity.
    #[must_use]
    pub const fn column_digest(&self) -> [u8; 32] {
        self.column_digest
    }

    /// All three truth outcomes on the exact execution signal column.
    #[must_use]
    pub const fn summary(&self) -> crate::expression::Summary {
        self.summary
    }
    /// Distinct actual IST sessions with at least one definitely true signal.
    #[must_use]
    pub const fn support_sessions(&self) -> u64 {
        self.support_sessions
    }
}

/// One authenticated complete program coordinate population.
#[derive(Debug)]
pub struct ValidatedExpressionGridV1<'a> {
    evaluated: &'a EvaluatedExpressionGridV1,
    digest: [u8; 32],
    offsets: Vec<Option<usize>>,
}

impl ValidatedExpressionGridV1<'_> {
    /// Digest of all program, source, policy, coordinate and cell bytes.
    #[must_use]
    pub const fn evaluation_digest(&self) -> [u8; 32] {
        self.digest
    }

    /// One canonical coordinate after the complete integrity pass.
    #[must_use]
    pub fn cell(&self, ordinal: usize) -> Option<&Cell> {
        self.evaluated.grid.cells.get(ordinal)
    }

    /// Evaluation object whose cells this capability authenticates.
    #[must_use]
    pub const fn evaluation(&self) -> &EvaluatedExpressionGridV1 {
        self.evaluated
    }
}

/// Exact execution-policy outcome of one program coordinate.
/// It carries no institutional statistical or cash-membership approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionDispositionV1 {
    selected: Option<SelectedExpressionExitV1>,
    refusal: ExecutionRefusalBitsV1,
    ordinal: usize,
    coordinate: Chosen,
    identity: [u8; 32],
}

impl ExpressionDispositionV1 {
    /// Selected capability when this coordinate meets the frozen grid policy.
    #[must_use]
    pub const fn selected(&self) -> Option<&SelectedExpressionExitV1> {
        self.selected.as_ref()
    }

    /// Exact named grid-policy reasons for refusal; zero means authorized.
    #[must_use]
    pub const fn refusal_bits(&self) -> ExecutionRefusalBitsV1 {
        self.refusal
    }

    /// Canonical coordinate ordinal, including policy-refused coordinates.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Full coordinate axes.
    #[must_use]
    pub const fn coordinate(&self) -> Chosen {
        self.coordinate
    }

    /// Program-aware identity of this exact coordinate's terminal outcome.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.identity
    }
}

/// A policy-authorized program coordinate, distinct from every AND capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedExpressionExitV1 {
    program: Arc<Expression>,
    run: RunId,
    resolution: [u8; 32],
    evaluation: [u8; 32],
    coordinate: Chosen,
    cell: Cell,
    identity: [u8; 32],
}

impl SelectedExpressionExitV1 {
    /// Complete executable program; never reconstruct from referenced bits.
    #[must_use]
    pub fn program(&self) -> &Expression {
        &self.program
    }

    /// Training run under which this exact program was priced.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run
    }

    /// Exact frozen exit coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> Chosen {
        self.coordinate
    }

    /// Actual measured TRAINING cell.
    #[must_use]
    pub const fn training_cell(&self) -> &Cell {
        &self.cell
    }

    /// Identity binding program, complete evaluation, coordinate and decision.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.identity
    }
}

impl ResolvedGridViewV1<'_> {
    /// Price every resolved coordinate using the actual Boolean program.
    ///
    /// # Errors
    /// Refuses crosswired source/resolution identities, invalid program signal
    /// alignment or an incomplete exact-grid result. No local ladder is derived.
    pub(crate) fn evaluate_expression_with_attested(
        &self,
        attested: &AttestedTrainingV1<'_>,
        run: &ExpressionExecutionRunV1,
    ) -> Result<EvaluatedExpressionGridV1, String> {
        self.require_runtime_integrity().map_err(display)?;
        if attested.resolution_digest != self.digest {
            return Err("expression attestation belongs to another resolution".to_owned());
        }
        run.source
            .require_matches_terms(
                &self.instrument,
                self.feed_digest,
                self.commit_digest,
                self.training_digest,
                self.side(),
            )
            .map_err(display)?;
        let grid = crate::grid::evaluate_resolved_expression_policy_v1(
            attested.bars,
            attested.column,
            run.program(),
            attested.horizon,
            self.side(),
            self,
        )?;
        let mut support_sessions = 0_u64;
        let mut last_session = None;
        let summary =
            crate::expression::evaluate(attested.column, run.program(), |index, truth| {
                if truth == crate::expression::Truth::True {
                    let bar = attested
                        .bars
                        .get(index)
                        .ok_or("expression support source is outside execution")?;
                    let day = indicators::ist_day(bar.ts_micros);
                    if last_session != Some(day) {
                        support_sessions = support_sessions
                            .checked_add(1)
                            .ok_or("expression support session count overflow")?;
                        last_session = Some(day);
                    }
                }
                Ok::<(), &'static str>(())
            })
            .map_err(|why| format!("expression source summary refused: {why:?}"))?;
        if summary.hits != grid.signals {
            return Err("expression exact truth summary differs from grid signals".to_owned());
        }
        Ok(EvaluatedExpressionGridV1 {
            resolution: self.digest,
            run: run.clone(),
            horizon: attested.horizon,
            column_digest: attested.column_digest,
            evaluation_spec: attested.evaluation_spec,
            side: self.side(),
            grid,
            summary,
            support_sessions,
        })
    }

    /// Authenticate the complete program population once before cell lookups.
    ///
    /// # Errors
    /// Refuses changed policy/ladders, incomplete/reordered cells, refused paths
    /// and allocation failure before returning a coordinate authority.
    pub(crate) fn validate_expression_evaluation<'a>(
        &self,
        evaluated: &'a EvaluatedExpressionGridV1,
    ) -> Result<ValidatedExpressionGridV1<'a>, String> {
        self.require_runtime_integrity().map_err(display)?;
        if evaluated.resolution != self.digest || evaluated.side != self.side() {
            return Err("expression evaluation belongs to another resolution".to_owned());
        }
        self.validate_complete_grid(&evaluated.grid)
            .map_err(display)?;
        Ok(ValidatedExpressionGridV1 {
            evaluated,
            digest: evaluation_digest(evaluated),
            offsets: self.coordinate_row_offsets().map_err(display)?,
        })
    }

    /// Classify any exact coordinate; selection does not erase refused cells.
    ///
    /// # Errors
    /// Refuses foreign validation or an unknown coordinate before policy checks.
    pub(crate) fn classify_expression_coordinate(
        &self,
        validated: &ValidatedExpressionGridV1<'_>,
        coordinate: Chosen,
    ) -> Result<ExpressionDispositionV1, String> {
        if validated.evaluated.resolution != self.digest
            || !self.chosen_axes_are_in_bounds(coordinate)
        {
            return Err(
                "expression coordinate/resolution is not this validated population".to_owned(),
            );
        }
        let ordinal = self
            .canonical_coordinate_ordinal(&validated.offsets, coordinate)
            .map_err(display)?
            .ok_or("expression coordinate is absent")?;
        let cell = validated
            .cell(ordinal)
            .ok_or("expression cell ordinal is absent")?;
        if Chosen::from_cell(cell) != coordinate {
            return Err("expression coordinate ordinal changed".to_owned());
        }
        let refusal = self.execution_refusal_bits(cell);
        let mut hasher = super::Hasher::new();
        hasher.update(b"brutex.runner.expression-disposition.v1\0");
        hasher.update(&validated.digest);
        super::put_usize(&mut hasher, ordinal);
        super::put_chosen(&mut hasher, coordinate);
        super::put_cell(&mut hasher, cell);
        super::put_u64(&mut hasher, refusal.bits());
        let identity = hasher.finalize();
        let selected = refusal.is_empty().then(|| SelectedExpressionExitV1 {
            program: Arc::clone(&validated.evaluated.run.program),
            run: validated.evaluated.run_id(),
            resolution: self.digest,
            evaluation: validated.digest,
            coordinate,
            cell: *cell,
            identity,
        });
        Ok(ExpressionDispositionV1 {
            selected,
            refusal,
            ordinal,
            coordinate,
            identity,
        })
    }

    /// Materialize one authenticated cell, including policy-refused comparison
    /// cells. Its exact rows must reproduce every measured cell field.
    ///
    /// # Errors
    /// Refuses foreign source/column/program or a replay mismatch. This grants
    /// observation evidence only, not admission of a policy-refused coordinate.
    pub(crate) fn materialize_expression_coordinate(
        &self,
        attested: &AttestedTrainingV1<'_>,
        validated: &ValidatedExpressionGridV1<'_>,
        ordinal: usize,
    ) -> Result<Vec<TradeRow>, String> {
        let evaluation = validated.evaluated;
        if evaluation.resolution != self.digest
            || attested.resolution_digest != self.digest
            || attested.column_digest != evaluation.column_digest
            || attested.horizon != evaluation.horizon
            || attested.evaluation_spec != evaluation.evaluation_spec
        {
            return Err(
                "expression replay does not match authenticated training inputs".to_owned(),
            );
        }
        let cell = validated
            .cell(ordinal)
            .ok_or("expression replay ordinal is absent")?;
        crate::grid::materialize_expression_cell(
            attested.bars,
            attested.column,
            evaluation.program(),
            evaluation.horizon,
            evaluation.side,
            &evaluation.grid,
            cell,
        )
    }
}

fn evaluation_digest(evaluated: &EvaluatedExpressionGridV1) -> [u8; 32] {
    let mut hasher = super::Hasher::new();
    hasher.update(b"brutex.runner.expression-grid.v1\0");
    hasher.update(&evaluated.resolution);
    hasher.update(&evaluated.run_id().bytes());
    hasher.update(&evaluated.program().encode());
    super::put_u32(&mut hasher, evaluated.horizon.as_bars());
    hasher.update(&evaluated.column_digest);
    hasher.update(evaluated.evaluation_spec.fingerprint_v1().as_bytes());
    super::put_side(&mut hasher, evaluated.side);
    super::put_u64(&mut hasher, evaluated.grid.signals);
    super::put_u64(&mut hasher, evaluated.support_sessions);
    for count in [
        evaluated.summary.evaluated,
        evaluated.summary.hits,
        evaluated.summary.misses,
        evaluated.summary.unknown,
    ] {
        super::put_u64(&mut hasher, count);
    }
    super::put_levels(&mut hasher, evaluated.grid.stops.rungs());
    super::put_levels(&mut hasher, evaluated.grid.targets.rungs());
    super::put_levels(&mut hasher, evaluated.grid.trails.rungs());
    super::put_u64(&mut hasher, evaluated.grid.refused_paths);
    super::put_usize(&mut hasher, evaluated.grid.cells.len());
    for cell in &evaluated.grid.cells {
        super::put_cell(&mut hasher, cell);
    }
    hasher.finalize()
}

fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}

macro_rules! expression_resolution {
    ($resolution:ty) => {
        impl $resolution {
            /// Price every frozen coordinate using an exact Boolean program.
            ///
            /// # Errors
            /// Refuses source, column, side, policy or complete-grid mismatch.
            pub fn evaluate_expression_with_attested(
                &self,
                attested: &AttestedTrainingV1<'_>,
                run: &ExpressionExecutionRunV1,
            ) -> Result<EvaluatedExpressionGridV1, String> {
                self.view().evaluate_expression_with_attested(attested, run)
            }

            /// Authenticate a complete program-aware coordinate population.
            ///
            /// # Errors
            /// Refuses changed, incomplete or foreign evaluation evidence.
            pub fn validate_expression_evaluation<'a>(
                &self,
                evaluated: &'a EvaluatedExpressionGridV1,
            ) -> Result<ValidatedExpressionGridV1<'a>, String> {
                self.view().validate_expression_evaluation(evaluated)
            }

            /// Classify the exact coordinate under its frozen execution policy.
            ///
            /// # Errors
            /// Refuses invalid coordinates or a foreign complete-grid capability.
            pub fn classify_expression_coordinate(
                &self,
                validated: &ValidatedExpressionGridV1<'_>,
                coordinate: Chosen,
            ) -> Result<ExpressionDispositionV1, String> {
                self.view()
                    .classify_expression_coordinate(validated, coordinate)
            }

            /// Replay one complete-grid coordinate without substituting a winner.
            ///
            /// # Errors
            /// Refuses source/column drift or any measured cell/row mismatch.
            pub fn materialize_expression_coordinate(
                &self,
                attested: &AttestedTrainingV1<'_>,
                validated: &ValidatedExpressionGridV1<'_>,
                ordinal: usize,
            ) -> Result<Vec<TradeRow>, String> {
                self.view()
                    .materialize_expression_coordinate(attested, validated, ordinal)
            }
        }
    };
}
expression_resolution!(ResolvedExitGridV1);
expression_resolution!(ResearchResolvedExitGridV1);

#[cfg(test)]
#[path = "expression_execution_tests.rs"]
mod tests;
