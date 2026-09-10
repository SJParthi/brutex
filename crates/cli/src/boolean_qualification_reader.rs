//! Detached full-comparison observer. Saved bytes never mint a source capability.
//! Cold checks reproduce source classifications, matrix hashes, observed ranks
//! and probability recurrences, but do not rerun bootstrap resampling. These
//! are authenticated historical producer observations, not fresh statistical
//! inference or current raw-market attestation.
use super::super::super::super::oos::reader::Reader as Later;
use super::super::reader::Admission;
use super::{Manifest, Row, display, persistence::Observation, project, wire};
use crate::boolean_qualification_plan::{ObservedPlan, ordered_identities};
use brutex_core::blake3::Hasher;
use runner::admission::{AdmissionEvidenceValuesV1, AdmissionPolicyV1, AdmissionVerdictV1};
use std::path::Path;

/// Every saved candidate, including losses, zeros and refused execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualificationRow {
    /// Original complete coordinate identity.
    pub original: [u8; 32],
    /// Exact later coordinate identity.
    pub later: [u8; 32],
    /// Index in the canonical family list.
    pub family: usize,
    /// Full grid coordinate offset.
    pub coordinate: usize,
    /// All common admission fields. PBO describes training; execution and
    /// profitability describe frozen-coordinate later observations.
    pub values: AdmissionEvidenceValuesV1,
    /// Every failed, refused and unavailable reason, without winner filtering.
    pub verdict: AdmissionVerdictV1,
    /// Conservative zero-aware probability and unrounded shared numeric facts.
    pub romano: [u64; 12],
    /// Full predeclared later fold record, never an authoring token.
    pub folds: Option<runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1>,
}

/// Authenticated fixed-training qualification with retained saved ancestors.
pub struct Reader {
    observation: Observation,
    original: Admission,
    later: Vec<Later>,
    manifest: Manifest,
    rows: Vec<Row>,
    plan: ObservedPlan,
    admitted_bytes: u64,
    consistency: Option<crate::index_consistency_store::Reader>,
}
impl Reader {
    /// Authenticate this complete record and all original/later ancestors under
    /// one aggregate serialized-byte admission. No raw source is re-attested.
    /// # Errors
    /// Refuses missing, changed, foreign, inconsistent or oversized evidence.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let (observation, (manifest, rows)) =
            Observation::open(root, super::NAMESPACE, identity, max_bytes, |raw| {
                wire::decode(raw, identity)
            })?;
        let mut remaining = charge(max_bytes, observation.body_bytes())?;
        let original = Admission::open(root, manifest.original, remaining)?;
        if original.completion_digest() != manifest.original_pin
            || original.row_count() != manifest.count
            || original.policy() != manifest.policy
        {
            return Err("qualification original admission link differs".into());
        }
        remaining = charge(remaining, original.body_bytes())?;
        // Charge the entire already-opened original ancestry before admitting
        // another reader, including every catalog retained by statistics.
        remaining = remaining
            .checked_sub(original.statistics().admitted_bytes())
            .ok_or("qualification original ancestry exceeds admission")?;
        let mut later = Vec::new();
        later
            .try_reserve_exact(manifest.later.len())
            .map_err(display)?;
        if original.statistics().sources().len() != manifest.later.len() {
            return Err("qualification family population differs".into());
        }
        let plan = verify_original_plan(&manifest, &original)?;
        for (link, source) in manifest.later.iter().zip(original.statistics().sources()) {
            let saved = Later::open(root, link.0, remaining)?;
            if saved.completion_digest() != link.1
                || saved.coordinate_count() != link.2
                || saved.parent().identity() != source.identity
                || saved.parent().completion_digest() != source.completion
            {
                return Err("qualification later/original full-family linkage differs".into());
            }
            remaining = charge(remaining, saved.body_bytes())?;
            // The original catalog is retained twice: by admission and by OOS.
            remaining = charge(remaining, saved.parent().body_bytes())?;
            later.push(saved);
        }
        let rung = usize::try_from(manifest.allocation.rung()).map_err(display)?;
        if ordered_identities(later.iter().map(|saved| saved.summary().source))
            != plan.expected_sources(rung)?
        {
            return Err(
                "qualification later full source identities differ from planned rung".into(),
            );
        }
        let consistency = if plan.index_policy().is_some() {
            let saved = crate::index_consistency_store::Reader::open(
                root,
                identity,
                observation.completion_digest(),
                remaining,
                manifest.count,
            )?;
            remaining = remaining
                .checked_sub(saved.admitted_bytes())
                .ok_or("qualification consistency receipt exceeds aggregate byte admission")?;
            Some(saved)
        } else {
            None
        };
        let result = Self {
            observation,
            original,
            later,
            manifest,
            rows,
            plan,
            consistency,
            admitted_bytes: max_bytes
                .checked_sub(remaining)
                .ok_or("qualification admitted bytes underflow")?,
        };
        result.check()?;
        result.require_current()?;
        Ok(result)
    }
    /// Exact qualification identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Exact completion pin required for every page.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Required daily/week assessment for a new qualification; historical plans
    /// return None and cannot be presented as having passed the new rule.
    /// # Errors
    /// Foreign pins, unknown coordinates or changed evidence refuse.
    pub fn index_consistency(
        &self,
        pin: [u8; 32],
        index: usize,
    ) -> Result<Option<&crate::index_consistency_store::Record>, String> {
        if pin != self.completion_digest() || index >= self.row_count() {
            return Err("index consistency qualification pin or setting differs".into());
        }
        self.require_current()?;
        self.consistency
            .as_ref()
            .map(|reader| reader.record(pin, index))
            .transpose()
    }
    /// Exact optional daily/week receipt; absent means a historical unassessed plan.
    #[must_use]
    pub fn index_consistency_receipt(&self) -> Option<crate::index_consistency_store::Receipt> {
        self.consistency
            .as_ref()
            .map(crate::index_consistency_store::Reader::receipt)
    }
    /// Exact serialized bytes admitted across all retained readers, including
    /// duplicated ancestor catalogs and112-byte receipts; not process RAM.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.admitted_bytes
    }
    /// Whether the finite bootstrap probability floor can reach this allocation.
    /// False is a valid recorded no-admission resolution, never increased draws.
    /// # Errors
    /// Refuses an invalid internal finite allocation.
    pub fn resolution_reachable(&self) -> Result<bool, String> {
        self.plan.resolution_reachable()
    }
    /// Predeclared finite campaign descriptor.
    #[must_use]
    pub const fn scope(&self) -> [u8; 32] {
        self.manifest.scope
    }
    /// Exact allocated timeframe unit.
    #[must_use]
    pub const fn unit(&self) -> [u8; 32] {
        self.manifest.unit
    }
    /// Full common research policy.
    #[must_use]
    pub const fn policy(&self) -> AdmissionPolicyV1 {
        self.manifest.policy
    }
    /// All coordinates, without success filtering.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// All original training receipts, with PBO diagnostic observations.
    #[must_use]
    pub const fn original(&self) -> &Admission {
        &self.original
    }
    /// All frozen-coordinate later families.
    #[must_use]
    pub fn later(&self) -> &[Later] {
        &self.later
    }
    /// Canonical zero-based rung allocation, across exactly eight timeframes.
    #[must_use]
    pub const fn allocation(&self) -> runner::family_allocation_v1::Allocation {
        self.manifest.allocation
    }
    /// Explicit draws, seed and aligned-session block length.
    #[must_use]
    pub const fn procedure(&self) -> [u64; 3] {
        self.manifest.procedure
    }
    /// Exact unscaled White reality-check and SPA probabilities for a separately
    /// predeclared search-wide projection. No ppm reconstruction is permitted.
    #[must_use]
    pub const fn family_probabilities(&self) -> [[u64; 2]; 2] {
        [
            [
                self.manifest.family_tests[0][2],
                self.manifest.family_tests[0][3],
            ],
            [
                self.manifest.family_tests[1][2],
                self.manifest.family_tests[1][3],
            ],
        ]
    }
    /// Declared contiguous later validation windows.
    #[must_use]
    pub const fn fold_count(&self) -> usize {
        self.manifest.folds
    }
    /// Recheck exact saved file generations for the entire retained ancestry.
    /// # Errors
    /// Any replaced, removed or altered evidence refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.with_current(|| {
            self.original.require_current()?;
            for later in &self.later {
                later.require_current()?;
            }
            if let Some(consistency) = &self.consistency {
                consistency.require_current()?;
            }
            Ok(())
        })
    }
    /// Return a bounded complete-comparison page, with full fold observations.
    /// # Errors
    /// Refuses incorrect pins, invalid pages, missing fields and changed evidence.
    pub fn rows(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<QualificationRow>, String> {
        if pin != self.completion_digest() || limit == 0 || limit > 256 || start > self.rows.len() {
            return Err("qualification page pin/range differs".into());
        }
        self.require_current()?;
        let end = start
            .checked_add(limit)
            .ok_or("qualification page overflow")?
            .min(self.rows.len());
        let result=self.rows.get(start..end).ok_or("qualification page outside rows")?.iter().map(|row|Ok(QualificationRow{
            original:row.original,later:row.later,family:row.family,coordinate:row.coordinate,
            values:row.projection.values(),verdict:row.projection.verdict(),romano:row.romano,
            folds:row.fold.as_ref().map(|raw|runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(raw,self.manifest.bounds.bytes,self.manifest.folds)).transpose()?,
        })).collect::<Result<Vec<_>,String>>()?;
        self.require_current()?;
        Ok(result)
    }
    fn check(&self) -> Result<(), String> {
        let measured = self.original.statistics().all_rows();
        let first = self
            .later
            .first()
            .ok_or("qualification later family absent")?;
        let mut returns = empty_source_matrix(&self.manifest)?;
        let mut at = 0;
        for (family, later) in self.later.iter().enumerate() {
            if later.sessions().len() != self.manifest.periods
                || later.sessions() != first.sessions()
            {
                return Err("qualification aligned later sessions differ".into());
            }
            verify_context(&self.plan, later)?;
            later.with_rows(|later_rows| {
                for (coordinate, after) in later_rows.iter().enumerate() {
                    let row = self
                        .rows
                        .get(at)
                        .ok_or("qualification saved coordinate missing")?;
                    let old = self
                        .original
                        .all_rows()
                        .get(at)
                        .ok_or("qualification original coordinate missing")?;
                    let original = measured
                        .get(at)
                        .ok_or("qualification training statistics missing")?;
                    if row.original != old.identity
                        || row.later != after.identity()
                        || row.family != family
                        || row.coordinate != coordinate
                        || original.source != family
                        || original.coordinate != coordinate
                    {
                        return Err("qualification saved coordinate linkage differs".into());
                    }
                    self.original.statistics().with_source(original, |before| {
                        if let Some(consistency) = &self.consistency {
                            let saved = consistency
                                .records()
                                .get(at)
                                .ok_or("index consistency coordinate missing")?;
                            self.check_consistency(saved, row, later, before, after)?;
                        }
                        verify_folds(
                            &self.manifest,
                            &self.plan,
                            later,
                            before,
                            after,
                            row.fold.as_deref(),
                        )?;
                        let expected = project(
                            &self.manifest,
                            &old.values,
                            before,
                            after,
                            row.romano,
                            row.fold.as_deref(),
                        )?;
                        if expected.canonical_bytes() != row.projection.canonical_bytes() {
                            return Err(
                                "qualification policy values differ from authenticated ancestors"
                                    .into(),
                            );
                        }
                        Ok(())
                    })?;
                    let mut values = Vec::new();
                    values
                        .try_reserve_exact(self.manifest.periods)
                        .map_err(display)?;
                    for (period, day) in after.periods().iter().zip(later.sessions()) {
                        if period.day() != *day {
                            return Err("qualification source period order differs".into());
                        }
                        values.push(period.return_paisa());
                    }
                    if values.len() != self.manifest.periods
                        || after.periods().len() != values.len()
                    {
                        return Err("qualification source period count differs".into());
                    }
                    returns.push(values);
                    at += 1;
                }
                Ok(())
            })?;
        }
        if at != self.rows.len() {
            return Err("qualification has extra coordinates".into());
        }
        verify_source_numbers(&self.manifest, &self.rows, &returns)?;
        Ok(())
    }
    fn check_consistency(
        &self,
        saved: &crate::index_consistency_store::Record,
        row: &Row,
        later: &Later,
        before: &super::super::super::super::BooleanCoordinateV1,
        after: &super::super::super::super::BooleanCoordinateV1,
    ) -> Result<(), String> {
        use crate::index_consistency::{Policy, evaluate};
        use crate::index_consistency_store::Binding;
        let expected = Binding {
            original: row.original,
            later: row.later,
            later_identity: later.identity(),
            later_completion: later.completion_digest(),
            source: later.summary().source,
            family: row.family as u64,
            coordinate: row.coordinate as u64,
        };
        let requested = self.plan.requested()?;
        let original = self.plan.training_requested()?;
        if saved.binding != expected
            || saved.evaluation.family != later.family()
            || saved.evaluation.first_day != original.first_day()
            || saved.evaluation.last_day != requested.last_day()
            || saved.training_session_count != before.periods().len()
            || saved.sessions.len() != before.periods().len() + after.periods().len()
            || saved
                .sessions
                .iter()
                .zip(before.periods().iter().chain(after.periods()))
                .any(|(day, original)| {
                    day.day != original.day()
                        || day.pessimistic_paisa != original.return_paisa()
                        || day.trades != original.trades()
                })
        {
            return Err(
                "index consistency differs from exact saved later source/coordinate/days".into(),
            );
        }
        // One cold validation at reader admission. Subsequent bounded pages use
        // the retained immutable rows and never reevaluate each displayed row.
        for (assessment, span, sessions) in [
            (
                &saved.training,
                (original.first_day(), original.last_day()),
                saved
                    .sessions
                    .get(..saved.training_session_count)
                    .ok_or("index original day extent missing")?,
            ),
            (
                &saved.later,
                (requested.first_day(), requested.last_day()),
                saved
                    .sessions
                    .get(saved.training_session_count..)
                    .ok_or("index later day extent missing")?,
            ),
            (
                &saved.evaluation,
                (original.first_day(), requested.last_day()),
                saved.sessions.as_slice(),
            ),
        ] {
            let evaluated = evaluate(Policy::V1, later.family(), span.0, span.1, sessions, false);
            if evaluated.canonical_bytes() != assessment.canonical_bytes() {
                return Err("index consistency saved assessment differs from authenticated daily observations".into());
            }
        }
        Ok(())
    }
}

fn verify_original_plan(manifest: &Manifest, original: &Admission) -> Result<ObservedPlan, String> {
    let plan = wire::validate_plan(manifest)?;
    let sources = original.statistics().sources();
    let keys: Vec<_> = sources
        .iter()
        .map(|source| source.family.instrument())
        .collect();
    let scope =
        runner::research_family::ResearchScopeV1::new(&keys, keys.len()).map_err(display)?;
    if scope.digest() != plan.scope_digest() {
        return Err("qualification original full family scope differs from plan".into());
    }
    let rung = usize::try_from(manifest.allocation.rung()).map_err(display)?;
    if ordered_identities(sources.iter().map(|source| source.identity))
        != plan.expected_catalogs(rung)?
    {
        return Err(
            "qualification original full catalog identities differ from planned rung".into(),
        );
    }
    Ok(plan)
}
fn charge(left: u64, body: u64) -> Result<u64, String> {
    left.checked_sub(
        body.checked_add(112)
            .ok_or("qualification ancestry size overflow")?,
    )
    .ok_or_else(|| "qualification aggregate saved ancestry exceeds admission".into())
}

fn empty_source_matrix(m: &Manifest) -> Result<Vec<Vec<i64>>, String> {
    let bytes = m
        .count
        .checked_mul(m.periods)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| {
            m.count
                .checked_mul(size_of::<Vec<i64>>())
                .and_then(|extra| n.checked_add(extra))
        })
        .ok_or("qualification source matrix size overflow")?;
    if bytes as u64 > m.bounds.memory {
        return Err("qualification source audit matrix exceeds admission".into());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(m.count).map_err(display)?;
    Ok(rows)
}

fn verify_source_numbers(m: &Manifest, rows: &[Row], returns: &[Vec<i64>]) -> Result<(), String> {
    let audit = runner::bootstrap_zero_v2::audit_sources(
        returns,
        usize::try_from(m.procedure[0]).map_err(display)?,
        m.procedure[1],
        usize::try_from(m.procedure[2]).map_err(display)?,
        runner::bootstrap_zero_v2::Bounds {
            max_work: m.bounds.work,
            max_bytes: m.bounds.memory,
        },
    )
    .map_err(|why| format!("qualification source numerical audit refused: {why:?}"))?;
    if audit.family_digest() != m.family_digest
        || audit.shared_digest() != m.shared_digest
        || audit.statistics().len() != rows.len()
    {
        return Err("qualification complete return matrix/procedure identity differs".into());
    }
    for (row, statistic) in rows.iter().zip(audit.statistics()) {
        let class = match statistic {
            None => 2,
            Some(bits) if f64::from_bits(*bits) > 0.0 => 1,
            Some(_) => 3,
        };
        if row.romano[0] != class || statistic.is_some_and(|bits| bits != row.romano[6]) {
            return Err("qualification probability belongs to another source statistic".into());
        }
    }
    Ok(())
}

fn verify_folds(
    m: &Manifest,
    plan: &ObservedPlan,
    later: &Later,
    before: &super::BooleanCoordinateV1,
    after: &super::BooleanCoordinateV1,
    raw: Option<&[u8]>,
) -> Result<(), String> {
    if raw.is_some() != before.execution_refusal_bits().is_empty() {
        return Err(
            "qualification fold presence differs from original selected disposition".into(),
        );
    }
    let Some(raw) = raw else {
        return Ok(());
    };
    let saved=runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(raw,m.bounds.bytes,m.folds)?;
    wire::validate_windows(plan, &saved)?;
    let grid = later
        .parent()
        .grids()
        .iter()
        .find(|grid| grid.policy.side() == before.side())
        .ok_or("qualification original side grid absent")?;
    let mut selected = Hasher::new();
    selected.update(b"brutex-boolean-candidate-coordinate-v1\0");
    selected.update(&later.parent().identity());
    selected.update(&later.family().encode());
    selected.update(&saved.selected_digest());
    if saved.training_run_id() != before.run_id()
        || saved.later_run_id() != after.run_id()
        || saved.ordinal() != before.ordinal()
        || saved.execution_refusal_bits() != after.execution_refusal_bits()
        || saved.folds().len() != m.folds
        || saved.resolution_digest() != grid.resolution
        || selected.finalize() != before.identity()
        || saved.training_last_day() != day(grid.last_micros)
        || saved.anchor_digest() == [0; 32]
        || saved.later_digest() == [0; 32]
    {
        return Err("qualification saved fold execution identities differ".into());
    }
    let mut periods = after.periods().iter().peekable();
    for fold in saved.folds() {
        let mut total = (0_u64, 0_u64, 0_u64, 0_i64);
        while let Some(period) = periods.peek() {
            if period.day() > fold.window.last_day() {
                break;
            }
            if period.day() < fold.window.first_day() {
                return Err("qualification fold omits later session".into());
            }
            total.0 = total
                .0
                .checked_add(1)
                .ok_or("qualification session overflow")?;
            total.1 = total
                .1
                .checked_add(period.trades())
                .ok_or("qualification trade overflow")?;
            total.2 = total
                .2
                .checked_add(period.wins())
                .ok_or("qualification win overflow")?;
            total.3 = total
                .3
                .checked_add(period.return_paisa())
                .ok_or("qualification fold paisa overflow")?;
            periods.next();
        }
        if total != (fold.sessions, fold.trades, fold.wins, fold.return_paisa) {
            return Err("qualification fold numbers do not conserve all later sessions".into());
        }
    }
    if periods.next().is_some() || saved.aggregate_oos_paisa()? != after.cell().pessimistic {
        return Err("qualification folds do not cover all later returns".into());
    }
    Ok(())
}

fn verify_context(plan: &ObservedPlan, later: &Later) -> Result<(), String> {
    let requested = plan.requested()?;
    let summary = later.summary();
    let first = pull::session::Day::new(summary.from.0, summary.from.1, 1).map_err(display)?;
    let last = pull::session::Day::new(summary.to.0, summary.to.1, 1)
        .map_err(display)?
        .end_of_month();
    let training = plan.training_requested()?;
    if i64::from(first.days_from_epoch()) != requested.first_day()
        || i64::from(last.days_from_epoch()) != requested.last_day()
        || crate::boolean_campaign::program_digest(later.programs()) != plan.program_digest()
        || later.programs().len() as u64 != plan.program_count()
        || day(summary.first_micros) < requested.first_day()
        || day(summary.last_micros) > requested.last_day()
        || later.parent().grids().iter().any(|grid| {
            u64::from(grid.horizon_bars) != plan.horizon()
                || day(grid.first_micros) < training.first_day()
                || day(grid.last_micros) > training.last_day()
        })
    {
        return Err(
            "qualification saved catalog/grid/date context differs from declared plan".into(),
        );
    }
    Ok(())
}
fn day(micros: i64) -> i64 {
    // Exact Euclidean IST mapping without adding the offset to a wide timestamp.
    micros.div_euclid(86_400_000_000)
        + (micros.rem_euclid(86_400_000_000) + 19_800_000_000) / 86_400_000_000
}
