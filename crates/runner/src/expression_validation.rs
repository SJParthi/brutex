//! Fixed-training validation over predeclared later civil windows.
//!
//! This is not expanding-prefix retraining. One opaque original program,
//! direction and exit coordinate stays fixed through the complete later run.
//! The storage caller retains source/calendar receipts; these numeric objects
//! cannot attest exchange-calendar completeness or authorize legacy selection.

use super::super::{ExecutionRefusalBitsV1, SelectedExpressionExitV1};
use super::{EvaluatedExpressionOosV1, ExpressionTrainingAnchorV1, ResearchResolvedExitGridV1};
use crate::grid::{Chosen, TradeRow};
use brutex_core::blake3::{Hasher, hash};

#[path = "expression_validation_codec.rs"]
mod codec;
pub use codec::ObservedFixedTrainingFoldsV1;

#[cfg(test)]
#[path = "expression_validation_tests.rs"]
mod tests;

/// Inclusive IST civil-day bounds, fixed before later-price evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaterSessionWindowV1 {
    first: i64,
    last: i64,
}
impl LaterSessionWindowV1 {
    /// # Errors
    /// Refuses reversed inclusive bounds.
    pub fn new(first: i64, last: i64) -> Result<Self, String> {
        if first > last {
            return Err("fixed-training window is reversed".into());
        }
        Ok(Self { first, last })
    }
    /// Inclusive first IST day since the Unix epoch.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first
    }
    /// Inclusive last IST day since the Unix epoch.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last
    }
}

/// Original sealed training context and an exact later-date partition.
/// Construction is O(folds), with an explicit allocation admission.
#[derive(Clone, Debug)]
pub struct FixedTrainingFoldPlanV1 {
    anchor: ExpressionTrainingAnchorV1,
    side: crate::excursion::Side,
    requested: LaterSessionWindowV1,
    windows: Vec<LaterSessionWindowV1>,
    training_last_day: i64,
    digest: [u8; 32],
}
impl FixedTrainingFoldPlanV1 {
    /// Freeze an exact contiguous partition strictly after original training.
    /// No window may be omitted, overlapped, or inferred from later returns.
    /// # Errors
    /// Refuses foreign training, noncontiguous dates or physical fold ceilings.
    pub fn new(
        resolved: &ResearchResolvedExitGridV1,
        anchor: &ExpressionTrainingAnchorV1,
        requested: LaterSessionWindowV1,
        windows: &[LaterSessionWindowV1],
        max_folds: usize,
    ) -> Result<Self, String> {
        resolved
            .view()
            .require_runtime_integrity()
            .map_err(display)?;
        if anchor.digest() != anchor.seal() || resolved.digest() != anchor.resolution_digest() {
            return Err("fixed-training plan has foreign training authority".into());
        }
        if windows.is_empty() || windows.len() > max_folds {
            return Err("fixed-training fold count exceeds physical admission".into());
        }
        let training_last_day = day(resolved.training_last_ts_micros());
        validate_windows(requested, windows, training_last_day)?;
        let mut retained = Vec::new();
        retained.try_reserve_exact(windows.len()).map_err(display)?;
        retained.extend_from_slice(windows);
        let mut h = Hasher::new();
        h.update(b"brutex.fixed-training-fold-plan.v1\0");
        h.update(&anchor.digest());
        h.update(&resolved.digest());
        h.update(&training_last_day.to_le_bytes());
        for window in std::iter::once(&requested).chain(windows) {
            h.update(&window.first.to_le_bytes());
            h.update(&window.last.to_le_bytes());
        }
        Ok(Self {
            anchor: anchor.clone(),
            side: resolved.side(),
            requested,
            windows: retained,
            training_last_day,
            digest: h.finalize(),
        })
    }
    /// Exact original context and declared partition identity.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Full requested later civil interval.
    #[must_use]
    pub const fn requested(&self) -> LaterSessionWindowV1 {
        self.requested
    }
    /// Ordered contiguous fold windows, including windows that might later refuse.
    #[must_use]
    pub fn windows(&self) -> &[LaterSessionWindowV1] {
        &self.windows
    }
    /// Exact original full-program anchor.
    #[must_use]
    pub const fn anchor(&self) -> &ExpressionTrainingAnchorV1 {
        &self.anchor
    }

    /// Index the complete later run once, before materializing its coordinates.
    /// O(bars + folds) work and O(bars + folds) additional memory; subsequent
    /// trade-to-fold mapping is a fixed index lookup. The byte limit covers this
    /// mapping and session counts, not retained market/candidate storage.
    /// # Errors
    /// Refuses foreign OOS, outside dates, empty actual folds or mapping ceilings.
    pub fn bind<'a>(
        &'a self,
        later: &'a EvaluatedExpressionOosV1<'a>,
        mapping_byte_cap: u64,
    ) -> Result<BoundFixedTrainingFoldsV1<'a>, String> {
        if later.anchor != self.anchor || later.side != self.side {
            return Err("fixed-training folds have foreign later authority".into());
        }
        let bytes = later
            .bars
            .len()
            .checked_mul(size_of::<usize>())
            .and_then(|n| {
                self.windows
                    .len()
                    .checked_mul(size_of::<u64>())
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or("fixed-training mapping byte overflow")?;
        if u64::try_from(bytes).map_err(display)? > mapping_byte_cap {
            return Err("fixed-training mapping exceeds byte admission".into());
        }
        let mut mapping = Vec::new();
        mapping
            .try_reserve_exact(later.bars.len())
            .map_err(display)?;
        let mut sessions = Vec::new();
        sessions
            .try_reserve_exact(self.windows.len())
            .map_err(display)?;
        sessions.resize(self.windows.len(), 0_u64);
        let mut fold = 0;
        let mut last_accepted = None;
        for (index, bar) in later.bars.iter().enumerate() {
            let actual = day(bar.ts_micros);
            while self
                .windows
                .get(fold)
                .is_some_and(|window| actual > window.last)
            {
                fold += 1;
            }
            let window = self
                .windows
                .get(fold)
                .ok_or("later bar outside declared folds")?;
            if actual < window.first {
                return Err("later bar precedes declared fold".into());
            }
            mapping.push(fold);
            if later.column.accepts(index) && last_accepted != Some(actual) {
                let count = sessions.get_mut(fold).ok_or("fold session slot absent")?;
                *count = count.checked_add(1).ok_or("fold session count overflow")?;
                last_accepted = Some(actual);
            }
        }
        if sessions.contains(&0) {
            return Err("fixed-training fold has no accepted actual session".into());
        }
        Ok(BoundFixedTrainingFoldsV1 {
            plan: self,
            later,
            mapping,
            sessions,
        })
    }
}

/// Shared mapping for all original coordinates on one immutable later run.
pub struct BoundFixedTrainingFoldsV1<'a> {
    plan: &'a FixedTrainingFoldPlanV1,
    later: &'a EvaluatedExpressionOosV1<'a>,
    mapping: Vec<usize>,
    sessions: Vec<u64>,
}
impl BoundFixedTrainingFoldsV1<'_> {
    /// Materialize once through the existing exact exit kernel, then partition.
    /// The returned rows are the same rows the storage producer must save.
    /// Zero-trade folds are measured zero outcomes, never profitable folds.
    /// Existing later execution refusal bits remain attached without relaxation.
    /// # Errors
    /// Refuses changed original selection, wrong ordinals or incomplete replay.
    pub fn materialize_coordinate(
        &self,
        selected: &SelectedExpressionExitV1,
        ordinal: usize,
    ) -> Result<(Vec<TradeRow>, FixedTrainingFoldProjectionV1), String> {
        let anchor = &self.plan.anchor;
        if selected.program() != anchor.program()
            || selected.run_id() != anchor.run_id()
            || selected.resolution != anchor.resolution
            || selected.evaluation != anchor.evaluation
        {
            return Err("fixed-training original selected authority differs".into());
        }
        let cell = self
            .later
            .grid()
            .cells
            .get(ordinal)
            .ok_or("fixed-training coordinate absent")?;
        if Chosen::from_cell(cell) != selected.coordinate() {
            return Err("fixed-training coordinate axes differ".into());
        }
        let trades = self.later.materialize(ordinal)?;
        let folds = partition(
            self.plan,
            &self.mapping,
            &self.sessions,
            self.later.bars,
            &trades,
        )?;
        let data = ProjectionData {
            plan: self.plan.digest,
            anchor: anchor.digest(),
            selected: selected.digest(),
            training_run: anchor.run_id().bytes(),
            later: self.later.digest(),
            later_run: self.later.run_id().bytes(),
            resolution: anchor.resolution_digest(),
            ordinal: u64::try_from(ordinal).map_err(display)?,
            requested: self.plan.requested,
            training_last_day: self.plan.training_last_day,
            refusal: self
                .later
                .refusal_bits(ordinal)
                .ok_or("later refusal coordinate absent")?,
            folds,
        };
        let projection = FixedTrainingFoldProjectionV1 { data };
        let totals = projection.totals()?;
        if totals.trades != cell.trades
            || totals.wins != cell.wins
            || totals.return_paisa != cell.pessimistic
        {
            return Err("fixed-training fold totals disagree with exact later cell".into());
        }
        Ok((trades, projection))
    }
}

/// One complete later window's exact observed outcome, including zero trades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedTrainingFoldV1 {
    /// Predeclared inclusive IST dates.
    pub window: LaterSessionWindowV1,
    /// Actual sessions with at least one accepted execution bar.
    pub sessions: u64,
    /// Saved exact-coordinate trades in this window.
    pub trades: u64,
    /// Strictly positive pessimistic trades.
    pub wins: u64,
    /// Checked sum of pessimistic per-unit paisa, with costs excluded.
    pub return_paisa: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectionData {
    plan: [u8; 32],
    anchor: [u8; 32],
    selected: [u8; 32],
    training_run: [u8; 32],
    later: [u8; 32],
    later_run: [u8; 32],
    resolution: [u8; 32],
    ordinal: u64,
    requested: LaterSessionWindowV1,
    training_last_day: i64,
    refusal: ExecutionRefusalBitsV1,
    folds: Vec<FixedTrainingFoldV1>,
}

/// Opaque numeric proof derived only from the exact original and later kernels.
/// A cold decoder returns a different observation-only type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedTrainingFoldProjectionV1 {
    data: ProjectionData,
}
impl FixedTrainingFoldProjectionV1 {
    /// Full exact numeric record, in a new domain; allocation is explicitly bounded.
    /// # Errors
    /// Refuses physical byte ceilings or inconsistent internal totals.
    pub fn canonical_bytes(&self, max_bytes: u64) -> Result<Vec<u8>, String> {
        codec::encode(&self.data, max_bytes)
    }
    /// Identity of the complete canonical proof.
    /// # Errors
    /// Refuses a canonical allocation ceiling or inconsistent internal totals.
    pub fn digest(&self, max_bytes: u64) -> Result<[u8; 32], String> {
        Ok(hash(&self.canonical_bytes(max_bytes)?))
    }
    /// Predeclared partition identity.
    #[must_use]
    pub const fn plan_digest(&self) -> [u8; 32] {
        self.data.plan
    }
    /// Exact original selected program/side/coordinate capability identity.
    #[must_use]
    pub const fn selected_digest(&self) -> [u8; 32] {
        self.data.selected
    }
    /// Full original training anchor.
    #[must_use]
    pub const fn anchor_digest(&self) -> [u8; 32] {
        self.data.anchor
    }
    /// Complete later run evaluation identity.
    #[must_use]
    pub const fn later_digest(&self) -> [u8; 32] {
        self.data.later
    }
    /// Full directional original execution run.
    #[must_use]
    pub const fn training_run_id(&self) -> [u8; 32] {
        self.data.training_run
    }
    /// Full directional later execution run.
    #[must_use]
    pub const fn later_run_id(&self) -> [u8; 32] {
        self.data.later_run
    }
    /// Complete original resolved exit identity.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.data.resolution
    }
    /// Canonical original coordinate index.
    #[must_use]
    pub const fn ordinal(&self) -> u64 {
        self.data.ordinal
    }
    /// Exact zero-inclusive fold outcomes.
    #[must_use]
    pub fn folds(&self) -> &[FixedTrainingFoldV1] {
        &self.data.folds
    }
    /// Existing later execution-policy refusal bits, including zero-trade refusal.
    #[must_use]
    pub const fn execution_refusal_bits(&self) -> ExecutionRefusalBitsV1 {
        self.data.refusal
    }
    /// Each complete window evaluates the original selected strategy, even at zero hits.
    #[must_use]
    pub fn decided_folds(&self) -> u64 {
        self.data.folds.len() as u64
    }
    /// Windows whose exact fixed-coordinate pessimistic sum is strictly positive.
    #[must_use]
    pub fn profitable_oos_folds(&self) -> u64 {
        self.data
            .folds
            .iter()
            .filter(|fold| fold.return_paisa > 0)
            .count() as u64
    }
    /// Checked aggregate of every fold, including losses and zeros.
    /// # Errors
    /// Refuses signed integer overflow or inconsistent fold records.
    pub fn aggregate_oos_paisa(&self) -> Result<i64, String> {
        Ok(self.totals()?.return_paisa)
    }
    fn totals(&self) -> Result<Totals, String> {
        validate_data(&self.data)
    }
}

fn partition(
    plan: &FixedTrainingFoldPlanV1,
    mapping: &[usize],
    sessions: &[u64],
    bars: &[indicators::Candle],
    trades: &[TradeRow],
) -> Result<Vec<FixedTrainingFoldV1>, String> {
    let mut folds = Vec::new();
    folds
        .try_reserve_exact(plan.windows.len())
        .map_err(display)?;
    folds.extend(
        plan.windows
            .iter()
            .zip(sessions)
            .map(|(&window, &sessions)| FixedTrainingFoldV1 {
                window,
                sessions,
                trades: 0,
                wins: 0,
                return_paisa: 0,
            }),
    );
    let mut previous = None;
    for trade in trades {
        let entry = bars
            .get(trade.entry_bar)
            .ok_or("fold entry source absent")?;
        let exit = bars.get(trade.exit_bar).ok_or("fold exit source absent")?;
        let first = mapping
            .get(trade.entry_bar)
            .copied()
            .ok_or("fold entry mapping absent")?;
        let last = mapping
            .get(trade.exit_bar)
            .copied()
            .ok_or("fold exit mapping absent")?;
        if first != last
            || trade.entry_micros != entry.ts_micros
            || trade.exit_micros != exit.ts_micros
            || day(entry.ts_micros) != day(exit.ts_micros)
            || trade.exit_bar < trade.entry_bar
            || previous.is_some_and(|index| trade.entry_bar <= index)
        {
            return Err("fixed-training trade crosses source/fold/session boundaries".into());
        }
        previous = Some(trade.exit_bar);
        let fold = folds.get_mut(first).ok_or("fold outcome absent")?;
        fold.trades = fold
            .trades
            .checked_add(1)
            .ok_or("fold trade count overflow")?;
        fold.wins = fold
            .wins
            .checked_add(u64::from(trade.worst > 0))
            .ok_or("fold win count overflow")?;
        fold.return_paisa = fold
            .return_paisa
            .checked_add(trade.worst)
            .ok_or("fold return overflow")?;
    }
    Ok(folds)
}
fn validate_windows(
    requested: LaterSessionWindowV1,
    windows: &[LaterSessionWindowV1],
    last_training: i64,
) -> Result<(), String> {
    validate_window_iter(requested, windows.iter().copied(), last_training)
}
fn validate_window_iter(
    requested: LaterSessionWindowV1,
    windows: impl Iterator<Item = LaterSessionWindowV1>,
    last_training: i64,
) -> Result<(), String> {
    let mut next = Some(requested.first);
    let mut last = None;
    for window in windows {
        if requested.first <= last_training
            || next != Some(window.first)
            || window.first > window.last
        {
            return Err(
                "fixed-training windows overlap training or fail exact contiguous coverage".into(),
            );
        }
        next = window.last.checked_add(1);
        last = Some(window.last);
    }
    if last != Some(requested.last) {
        return Err("fixed-training windows do not cover requested dates".into());
    }
    Ok(())
}
#[derive(Default)]
struct Totals {
    trades: u64,
    wins: u64,
    return_paisa: i64,
}
fn validate_data(data: &ProjectionData) -> Result<Totals, String> {
    validate_window_iter(
        data.requested,
        data.folds.iter().map(|fold| fold.window),
        data.training_last_day,
    )?;
    let mut total = Totals::default();
    for fold in &data.folds {
        if fold.sessions == 0
            || i128::from(fold.sessions)
                > i128::from(fold.window.last) - i128::from(fold.window.first) + 1
            || fold.wins > fold.trades
            || (fold.trades == 0 && fold.return_paisa != 0)
            || (fold.wins == 0 && fold.return_paisa > 0)
            || (fold.wins == fold.trades && fold.trades > 0 && fold.return_paisa <= 0)
        {
            return Err("fixed-training fold count/return hierarchy differs".into());
        }
        total.trades = total
            .trades
            .checked_add(fold.trades)
            .ok_or("aggregate fold trades overflow")?;
        total.wins = total
            .wins
            .checked_add(fold.wins)
            .ok_or("aggregate fold wins overflow")?;
        total.return_paisa = total
            .return_paisa
            .checked_add(fold.return_paisa)
            .ok_or("aggregate fold return overflow")?;
    }
    Ok(total)
}
const fn day(stamp: i64) -> i64 {
    const DAY: i64 = 86_400_000_000;
    stamp.div_euclid(DAY)
        + if stamp.rem_euclid(DAY) >= DAY - 19_800_000_000 {
            1
        } else {
            0
        }
}
fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}
