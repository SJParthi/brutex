//! Concrete Population V4/admission/Execution V2 authority for Selection V4.
//!
//! The three durable sources are opened and sampled under the common
//! `population-write.lock`.  A pair constructor is deliberate: the shared
//! cohort is derived from the exact NIFTY and BANKNIFTY V4 receipts while one
//! read generation is held, rather than accepted as a caller-authored value.
//! Every page reacquires that outer shared lock, asks both underlying readers
//! to revalidate their open-time file generations, and then checks every
//! population, sequence, strategy, payload and completion binding again.
//!
//! # Cost
//!
//! Opening scans the three underlying authorities and is O(stored records).
//! One page is O(requested rows) with a hard 256-row ceiling.  An indexed row
//! or completion lookup inside an already-open ledger is average O(1), not
//! worst-case O(1).  The join, hashing, open and full Selection V4 passes are
//! not O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::fs::File;
use std::path::{Path, PathBuf};

use runner::admission::AdmissionStatusV1;

use crate::admission_join::{
    AdmissionAuthoritativeLedger, AdmissionAuthoritativePageV1, AdmissionAuthorityIdentityV1,
};
use crate::execution_disposition_v2::{
    ExecutionCapabilityCompletionV2, ExecutionDispositionLedgerV2, ExecutionDispositionPageV2,
    ExecutionDispositionTagV2, ExecutionRowDispositionV2, MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2,
};
use crate::population::{InstrumentFamilyV1, MAX_PAGE_ROWS_V1, PopulationRowV1};
use crate::selection::SharedCohortIdentityV2;
use crate::selection_v4::{
    AdmissionAuthorityCountsV4, AdmissionAuthoritySnapshotV4, AdmissionSelectionRowV4,
    ExecutionAuthorityCountsV2, ExecutionAuthoritySnapshotV2, ExecutionSelectionRowV2,
    ExecutionSelectionStatusV2, PopulationAuthoritySnapshotV4, SelectionAuthorityPageV4,
    SelectionAuthoritySnapshotV4, SelectionAuthorityViewV4, SelectionJoinedRowV4,
    SelectionV4Refusal,
};

/// Read-only triple-authority view for one exact completed population.
#[derive(Debug)]
pub struct SelectionAuthorityLedgerV4 {
    population_id: [u8; 32],
    admissions: AdmissionAuthoritativeLedger,
    executions: ExecutionDispositionLedgerV2,
    execution_completion: ExecutionCapabilityCompletionV2,
    outer_lock: File,
    lock_path: PathBuf,
    snapshot: SelectionAuthoritySnapshotV4,
}

struct OpenAuthorityPartsV4 {
    population_id: [u8; 32],
    admissions: AdmissionAuthoritativeLedger,
    executions: ExecutionDispositionLedgerV2,
    execution_completion: ExecutionCapabilityCompletionV2,
    outer_lock: File,
    lock_path: PathBuf,
}

impl SelectionAuthorityLedgerV4 {
    /// Opens the canonical NIFTY/BANKNIFTY pair under one shared generation.
    ///
    /// The cohort is derived from the two exact V4 receipts.  There is no
    /// single-population constructor that could accept an invented peer or
    /// caller-authored cohort.
    ///
    /// # Errors
    ///
    /// Refuses equal identities, absent files or authorities, lock failure,
    /// noncanonical family order, different timeframes/cohorts, any upstream
    /// digest/count/matrix mismatch, or failure to release the opening lock.
    pub fn open_pair_read(
        root: &Path,
        nifty_population_id: [u8; 32],
        bank_nifty_population_id: [u8; 32],
    ) -> Result<(Self, Self), SelectionV4Refusal> {
        if nifty_population_id == bank_nifty_population_id {
            return Err(
                "Selection V4 NIFTY and BANKNIFTY authorities cannot share one population identity"
                    .to_owned(),
            );
        }
        let lock_path = root.join("results").join("population-write.lock");
        let outer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        outer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be shared-locked for Selection V4 pair open: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let nifty = OpenAuthorityPartsV4::open(
                root,
                nifty_population_id,
                independent_lock_handle(&lock_path)?,
                lock_path.clone(),
            )?;
            let bank_nifty = OpenAuthorityPartsV4::open(
                root,
                bank_nifty_population_id,
                independent_lock_handle(&lock_path)?,
                lock_path.clone(),
            )?;
            let nifty_v4 = nifty.admissions.snapshot().population_v4();
            let bank_v4 = bank_nifty.admissions.snapshot().population_v4();
            require_family(&nifty_v4.v3().v2(), InstrumentFamilyV1::Nifty)?;
            require_family(&bank_v4.v3().v2(), InstrumentFamilyV1::BankNifty)?;
            if nifty_v4.v3().v2().rung_seconds != bank_v4.v3().v2().rung_seconds {
                return Err(format!(
                    "Selection V4 pair has different timeframes: NIFTY={}s, BANKNIFTY={}s",
                    nifty_v4.v3().v2().rung_seconds,
                    bank_v4.v3().v2().rung_seconds
                ));
            }
            let cohort = SharedCohortIdentityV2::from_v4_receipts(&nifty_v4, &bank_v4)?;
            Ok((nifty.finish(&cohort)?, bank_nifty.finish(&cohort)?))
        })();
        release_outer(&outer_lock, &lock_path, opened)
    }

    fn page_locked(
        &mut self,
        offset: u64,
        limit: u64,
    ) -> Result<SelectionAuthorityPageV4, SelectionV4Refusal> {
        require_page_limit(limit)?;
        let current_execution =
            self.executions
                .completion(self.population_id)?
                .ok_or_else(|| {
                    format!(
                        "population {} lost its Execution V2 completion",
                        hex(&self.population_id)
                    )
                })?;
        if current_execution != self.execution_completion {
            return Err(format!(
                "population {} Execution V2 completion differs from this view's open-time snapshot",
                hex(&self.population_id)
            ));
        }
        let admission_page = self.admissions.page(offset, limit)?;
        let execution_page = self
            .executions
            .page(self.population_id, offset, limit)?
            .ok_or_else(|| {
                format!(
                    "population {} lost its Execution V2 page authority",
                    hex(&self.population_id)
                )
            })?;
        join_pages(
            &self.snapshot,
            offset,
            limit,
            admission_page,
            execution_page,
        )
    }
}

impl SelectionAuthorityViewV4 for SelectionAuthorityLedgerV4 {
    fn snapshot(&self) -> &SelectionAuthoritySnapshotV4 {
        &self.snapshot
    }

    fn page(
        &mut self,
        offset: u64,
        limit: u64,
    ) -> Result<SelectionAuthorityPageV4, SelectionV4Refusal> {
        self.outer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be shared-locked for a Selection V4 page: {why}",
                self.lock_path.display()
            )
        })?;
        let page = self.page_locked(offset, limit);
        release_outer(&self.outer_lock, &self.lock_path, page)
    }
}

impl OpenAuthorityPartsV4 {
    fn open(
        root: &Path,
        population_id: [u8; 32],
        outer_lock: File,
        lock_path: PathBuf,
    ) -> Result<Self, SelectionV4Refusal> {
        let admissions = AdmissionAuthoritativeLedger::open_read(root, population_id)?;
        let mut executions = ExecutionDispositionLedgerV2::open_read(root)?;
        let execution_completion = executions.completion(population_id)?.ok_or_else(|| {
            format!(
                "population {} has no complete Execution V2 authority",
                hex(&population_id)
            )
        })?;
        Ok(Self {
            population_id,
            admissions,
            executions,
            execution_completion,
            outer_lock,
            lock_path,
        })
    }

    fn finish(
        self,
        cohort: &SharedCohortIdentityV2,
    ) -> Result<SelectionAuthorityLedgerV4, SelectionV4Refusal> {
        let snapshot = build_snapshot(&self.admissions, &self.execution_completion, cohort)?;
        if snapshot.population.population_id != self.population_id {
            return Err("Selection V4 open parts changed population identity".to_owned());
        }
        Ok(SelectionAuthorityLedgerV4 {
            population_id: self.population_id,
            admissions: self.admissions,
            executions: self.executions,
            execution_completion: self.execution_completion,
            outer_lock: self.outer_lock,
            lock_path: self.lock_path,
            snapshot,
        })
    }
}

fn build_snapshot(
    admissions: &AdmissionAuthoritativeLedger,
    execution: &ExecutionCapabilityCompletionV2,
    cohort: &SharedCohortIdentityV2,
) -> Result<SelectionAuthoritySnapshotV4, SelectionV4Refusal> {
    let admission_snapshot = admissions.snapshot();
    let v4 = admission_snapshot.population_v4();
    let population = v4.v3().v2();
    let admission = admission_snapshot.admission();
    let identity = admission_snapshot.identity();
    let matrix = matrix_rows(execution.admission_execution_matrix().counts());
    require_execution_bindings(
        population.population_id,
        identity,
        admission,
        execution,
        matrix,
    )?;
    Ok(SelectionAuthoritySnapshotV4 {
        population: PopulationAuthoritySnapshotV4 {
            family: population.instrument_family,
            population_id: population.population_id,
            rung_seconds: population.rung_seconds,
            row_count: population.row_count,
            ordered_row_digest: population.ordered_row_digest,
            completion_digest: identity.population_v4_completion_digest,
            ranking_policy_digest: population.identities.ranking_policy_digest,
            cohort: *cohort,
        },
        admission: AdmissionAuthoritySnapshotV4 {
            population_id: admission.population_id(),
            population_v4_completion_digest: admission.population_v4_completion_digest(),
            completion_digest: identity.admission_completion_digest,
            decision_count: admission.decision_count(),
            counts: AdmissionAuthorityCountsV4 {
                admitted: admission.admitted_count(),
                rejected: admission.rejected_count(),
                unmeasured: admission.unmeasured_count(),
                refused: admission.refused_count(),
            },
        },
        execution: ExecutionAuthoritySnapshotV2 {
            population_id: execution.population_id(),
            population_v4_completion_digest: execution.population_v4_digest(),
            admission_completion_digest: execution.admission_completion_digest(),
            completion_digest: execution.completion_id(),
            disposition_count: execution.row_count(),
            counts: ExecutionAuthorityCountsV2 {
                authorized: execution.authorized_capability_count(),
                policy_refused: execution.policy_refused_count(),
            },
            admission_execution_counts: matrix,
        },
    })
}

fn require_execution_bindings(
    population_id: [u8; 32],
    admission_identity: AdmissionAuthorityIdentityV1,
    admission: &crate::admission_store::AdmissionCompletionReceiptV1,
    execution: &ExecutionCapabilityCompletionV2,
    matrix: [[u64; 2]; 4],
) -> Result<(), SelectionV4Refusal> {
    if execution.population_id() != population_id
        || admission.population_id() != population_id
        || execution.population_v4_digest() != admission_identity.population_v4_completion_digest
        || execution.admission_completion_digest() != admission_identity.admission_completion_digest
    {
        return Err(format!(
            "population {} execution authority binds different Population V4/admission completions",
            hex(&population_id)
        ));
    }
    if execution.row_count() != admission.decision_count() {
        return Err(format!(
            "population {} execution/admission row counts differ",
            hex(&population_id)
        ));
    }
    let admission_counts = [
        admission.admitted_count(),
        admission.rejected_count(),
        admission.unmeasured_count(),
        admission.refused_count(),
    ];
    for (index, expected) in admission_counts.into_iter().enumerate() {
        let row = matrix
            .get(index)
            .ok_or_else(|| "Execution V2 admission matrix row is absent".to_owned())?;
        if checked_sum(*row, "Execution V2 admission matrix row")? != expected {
            return Err(
                "Execution V2 matrix disagrees with admission completion counts".to_owned(),
            );
        }
    }
    let authorized = checked_sum(
        matrix.map(|row| row[0]),
        "Execution V2 authorized matrix column",
    )?;
    let policy_refused = checked_sum(
        matrix.map(|row| row[1]),
        "Execution V2 policy-refused matrix column",
    )?;
    if authorized != execution.authorized_capability_count()
        || policy_refused != execution.policy_refused_count()
    {
        return Err("Execution V2 matrix disagrees with terminal execution counts".to_owned());
    }
    Ok(())
}

fn join_pages(
    snapshot: &SelectionAuthoritySnapshotV4,
    offset: u64,
    limit: u64,
    admission_page: AdmissionAuthoritativePageV1,
    execution_page: ExecutionDispositionPageV2,
) -> Result<SelectionAuthorityPageV4, SelectionV4Refusal> {
    require_page_limit(limit)?;
    let expected = snapshot
        .population
        .row_count
        .saturating_sub(offset)
        .min(limit);
    let admission_len = u64::try_from(admission_page.rows.len())
        .map_err(|_| "Selection V4 admission page length does not fit u64".to_owned())?;
    let execution_len = u64::try_from(execution_page.len())
        .map_err(|_| "Selection V4 execution page length does not fit u64".to_owned())?;
    if admission_page.total != snapshot.population.row_count
        || execution_page.total_rows() != snapshot.population.row_count
        || admission_page.offset != offset
        || execution_page.offset() != offset
        || admission_len != expected
        || execution_len != expected
    {
        return Err(format!(
            "population {} triple-authority page is not exact at offset {offset}",
            hex(&snapshot.population.population_id)
        ));
    }
    if admission_page.authority.population_v4_completion_digest
        != snapshot.population.completion_digest
        || admission_page.authority.admission_completion_digest
            != snapshot.admission.completion_digest
        || execution_page.population_id() != snapshot.population.population_id
        || execution_page.completion_id() != snapshot.execution.completion_digest
    {
        return Err(format!(
            "population {} page was read under different completion authorities",
            hex(&snapshot.population.population_id)
        ));
    }
    let capacity = usize::try_from(expected)
        .map_err(|_| "Selection V4 page capacity does not fit usize".to_owned())?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|why| format!("Selection V4 page could not reserve {capacity} rows: {why}"))?;
    for (relative, (admission, execution)) in admission_page
        .rows
        .into_iter()
        .zip(execution_page.into_rows())
        .enumerate()
    {
        let relative = u64::try_from(relative)
            .map_err(|_| "Selection V4 relative page sequence does not fit u64".to_owned())?;
        let sequence = offset
            .checked_add(relative)
            .ok_or_else(|| "Selection V4 page sequence overflowed u64".to_owned())?;
        rows.push(join_row(
            snapshot,
            sequence,
            &admission.population,
            &admission.admission,
            &execution,
        )?);
    }
    Ok(SelectionAuthorityPageV4 {
        total: snapshot.population.row_count,
        offset,
        rows,
        population_v4_completion_digest: snapshot.population.completion_digest,
        admission_completion_digest: snapshot.admission.completion_digest,
        execution_completion_digest: snapshot.execution.completion_digest,
    })
}

fn join_row(
    snapshot: &SelectionAuthoritySnapshotV4,
    expected_sequence: u64,
    population: &PopulationRowV1,
    admission: &crate::admission_store::AdmissionDecisionRecordV1,
    execution: &ExecutionRowDispositionV2,
) -> Result<SelectionJoinedRowV4, SelectionV4Refusal> {
    let payload_digest = population.payload_digest()?;
    let admission_status = admission.verdict().status();
    let execution_status = map_execution_tag(execution.tag());
    let binding = RowBindingV4 {
        population_id: population.population_id,
        sequence: population.sequence,
        strategy_digest: population.strategy_digest,
        payload_digest,
        population_v4_digest: snapshot.population.completion_digest,
        admission_completion_digest: snapshot.admission.completion_digest,
        admission_status,
    };
    require_row_binding(
        expected_sequence,
        binding,
        RowBindingV4 {
            population_id: admission.population_id(),
            sequence: admission.row_sequence(),
            strategy_digest: admission.strategy_digest(),
            payload_digest: admission.row_payload_digest(),
            population_v4_digest: snapshot.population.completion_digest,
            admission_completion_digest: snapshot.admission.completion_digest,
            admission_status,
        },
        RowBindingV4 {
            population_id: execution.population_id(),
            sequence: execution.row_sequence(),
            strategy_digest: execution.strategy_digest(),
            payload_digest: execution.row_payload_digest(),
            population_v4_digest: execution.population_v4_digest(),
            admission_completion_digest: execution.admission_completion_digest(),
            admission_status: execution.admission_status(),
        },
    )?;
    let decision_digest = brutex_core::blake3::hash(&admission.to_bytes()?);
    Ok(SelectionJoinedRowV4 {
        population: *population,
        admission: AdmissionSelectionRowV4 {
            population_id: admission.population_id(),
            sequence: admission.row_sequence(),
            strategy_digest: admission.strategy_digest(),
            population_payload_digest: admission.row_payload_digest(),
            decision_digest,
            status: admission_status,
        },
        execution: ExecutionSelectionRowV2 {
            population_id: execution.population_id(),
            sequence: execution.row_sequence(),
            strategy_digest: execution.strategy_digest(),
            population_payload_digest: execution.row_payload_digest(),
            population_v4_completion_digest: execution.population_v4_digest(),
            admission_completion_digest: execution.admission_completion_digest(),
            admission_status: execution.admission_status(),
            disposition_digest: execution.disposition_id(),
            status: execution_status,
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowBindingV4 {
    population_id: [u8; 32],
    sequence: u64,
    strategy_digest: [u8; 32],
    payload_digest: [u8; 32],
    population_v4_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    admission_status: AdmissionStatusV1,
}

fn require_row_binding(
    expected_sequence: u64,
    population: RowBindingV4,
    admission: RowBindingV4,
    execution: RowBindingV4,
) -> Result<(), SelectionV4Refusal> {
    if population.sequence != expected_sequence {
        return Err(format!(
            "Selection V4 population sequence {} differs from expected {expected_sequence}",
            population.sequence
        ));
    }
    if admission != population {
        return Err(format!(
            "Selection V4 admission row at sequence {expected_sequence} differs from Population V4"
        ));
    }
    if execution != population {
        return Err(format!(
            "Selection V4 execution row at sequence {expected_sequence} differs from Population V4/admission authority"
        ));
    }
    Ok(())
}

const fn map_execution_tag(tag: ExecutionDispositionTagV2) -> ExecutionSelectionStatusV2 {
    match tag {
        ExecutionDispositionTagV2::Authorized => ExecutionSelectionStatusV2::Authorized,
        ExecutionDispositionTagV2::PolicyRefused => ExecutionSelectionStatusV2::PolicyRefused,
    }
}

const fn matrix_rows(counts: [u64; 8]) -> [[u64; 2]; 4] {
    [
        [counts[0], counts[1]],
        [counts[2], counts[3]],
        [counts[4], counts[5]],
        [counts[6], counts[7]],
    ]
}

fn require_page_limit(limit: u64) -> Result<(), SelectionV4Refusal> {
    if limit > MAX_PAGE_ROWS_V1 || limit > MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2 {
        return Err(format!(
            "Selection V4 page limit {limit} exceeds the fixed 256-row ceiling"
        ));
    }
    Ok(())
}

fn require_family(
    receipt: &crate::population::CompletionReceiptV2,
    expected: InstrumentFamilyV1,
) -> Result<(), SelectionV4Refusal> {
    if receipt.instrument_family != expected {
        return Err(format!(
            "Selection V4 pair expected {expected:?}, found {:?}",
            receipt.instrument_family
        ));
    }
    Ok(())
}

fn checked_sum<const N: usize>(values: [u64; N], label: &str) -> Result<u64, SelectionV4Refusal> {
    values.into_iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(value)
            .ok_or_else(|| format!("{label} overflowed u64"))
    })
}

fn release_outer<T>(
    lock: &File,
    path: &Path,
    result: Result<T, SelectionV4Refusal>,
) -> Result<T, SelectionV4Refusal> {
    let released = lock.unlock().map_err(|why| {
        format!(
            "{} Selection V4 outer shared lock could not be released: {why}",
            path.display()
        )
    });
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn independent_lock_handle(path: &Path) -> Result<File, SelectionV4Refusal> {
    File::open(path).map_err(|why| {
        format!(
            "{} could not be independently opened for a Selection V4 view: {why}",
            path.display()
        )
    })
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(char::from(
            DIGITS.get(usize::from(byte >> 4)).copied().unwrap_or(b'?'),
        ));
        out.push(char::from(
            DIGITS
                .get(usize::from(byte & 0x0f))
                .copied()
                .unwrap_or(b'?'),
        ));
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    reason = "real receipt-last integration fixtures fail loudly when their own canonical authority setup is invalid"
)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use pull::calendar::{DayKind, OPEN_MINUTE, kind_of};
    use pull::session::Day;
    use runner::admission::{
        AdmissionEvidenceV1, AdmissionEvidenceValuesV1, AdmissionPolicyDraftV1, AdmissionPolicyV1,
        AdmissionStatusV1, CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
    };
    use runner::excursion::Side;
    use runner::exit_grid_policy::{
        EvaluatedExitGridV1, ExecutionResolutionV1, ExecutionRunV1, ExecutionSeriesV1,
        ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
        RationalPercentileV1, ResolvedExitGridV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
    };
    use runner::grid::{Chosen, Ttp};
    use runner::identity::{Direction, Params, Run};
    use runner::outcome::Horizon;

    use crate::admission_store::{
        AdmissionAuthorityLedger, AdmissionCompletionReceiptV1, AdmissionDecisionRecordV1,
    };
    use crate::execution_capability::{ExecutionParametersV1, ExecutionStrategyCapabilityV1};
    use crate::execution_disposition_v2::{
        EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2, EXECUTION_DISPOSITION_ROW_STRIDE_V2,
        ExecutionDispositionLedgerV2, ExecutionRowDispositionV2, PreparedExecutionDispositionsV2,
    };
    use crate::population::{
        CompletionReceiptV4, InstrumentFamilyV1, LongShortExitGridIdentitiesV2,
        PopulationIdentitiesV2, PopulationLedger, PopulationRowV1, RequestedSpanIdentityV1,
        SideExitGridIdentityV2, TradeDirectionV1,
    };
    use crate::population_admission_writer::{
        CompletePopulationAuthorityV1, PopulationCellEvidenceV1, produce_population_admission_v1,
    };
    use crate::selection_v4::SelectionAuthorityViewV4;
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};

    use super::{
        ExecutionDispositionTagV2, ExecutionSelectionStatusV2, RowBindingV4,
        SelectionAuthorityLedgerV4, checked_sum, independent_lock_handle, map_execution_tag,
        matrix_rows, require_page_limit, require_row_binding,
    };

    static NEXT_LOCK: AtomicU64 = AtomicU64::new(0);
    static NEXT_STORE: AtomicU64 = AtomicU64::new(0);
    const TEST_FEED: &str = "selection-v4-authority-test-feed";
    const TEST_COMMIT: &str = "selection-v4-authority-test-commit";
    const TEST_CALENDAR_POLICY: [u8; 32] = [0xA5; 32];
    const HEADER_BYTES: u64 = 24;
    const COMPLETION_PAYLOAD_BYTES: usize = 512;
    const COMPLETION_POPULATION_OFFSET: usize = 32;
    const COMPLETION_POPULATION_V4_OFFSET: usize = 64;
    const COMPLETION_MATRIX_OFFSET: usize = 10 * 32 + 9 * 8;
    const ROW_POPULATION_OFFSET: usize = 32;

    const fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    const fn binding() -> RowBindingV4 {
        RowBindingV4 {
            population_id: digest(1),
            sequence: 7,
            strategy_digest: digest(2),
            payload_digest: digest(3),
            population_v4_digest: digest(4),
            admission_completion_digest: digest(5),
            admission_status: AdmissionStatusV1::Admitted,
        }
    }

    #[derive(Debug)]
    struct DurablePopulationFixture {
        population_id: [u8; 32],
        rows: Vec<PopulationRowV1>,
        decisions: Vec<AdmissionDecisionRecordV1>,
        population_v4: CompletionReceiptV4,
        admission: AdmissionCompletionReceiptV1,
        instrument: InstrumentKey,
        bars: Vec<indicators::Candle>,
        column: Column,
        long_grid: ResolvedExitGridV1,
        short_grid: ResolvedExitGridV1,
        ladder: Ladder,
        horizon: Horizon,
    }

    fn store_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-selection-v4-authority-{}-{tag}-{}",
            std::process::id(),
            NEXT_STORE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn clean_store(path: &Path) {
        let _ignored = fs::remove_dir_all(path);
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn instrument(symbol: &str) -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, symbol).expect("swept NSE spot-index fixture")
    }

    fn percentile() -> RationalPercentileV1 {
        RationalPercentileV1::new(1, 2).expect("one-half is a valid percentile")
    }

    fn exit_policy(side: Side) -> ExitGridPolicyV1 {
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            RungPlanV1::new(
                vec![percentile()],
                vec![percentile()],
                vec![percentile()],
                1,
            )
            .expect("one exact rung per dynamic axis"),
            RatioLimitsV1::new(1, 10_000, 1).expect("wide exact ratio interval"),
            1_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete one-minute side policy")
    }

    fn requested_span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("measured fixture month")
    }

    fn complete_calendar(rung_seconds: u32) -> CompleteCalendarReceiptV2 {
        let requested = requested_span();
        let first =
            Day::new(requested.from_year(), requested.from_month(), 1).expect("first fixture day");
        let last = Day::new(requested.to_year(), requested.to_month(), 1)
            .expect("last fixture month")
            .end_of_month();
        let first_day = i64::from(first.days_from_epoch());
        let last_day = i64::from(last.days_from_epoch());
        let width = i64::from(rung_seconds / 60);
        let mut timestamps = Vec::new();
        for day in first_day..=last_day {
            match kind_of(day) {
                DayKind::Open(session) => {
                    for window in session.windows.iter().take(usize::from(session.count)) {
                        for minute in window.from..=window.to {
                            let bucket =
                                i64::from(minute).saturating_sub(i64::from(OPEN_MINUTE)) / width;
                            let bucket_minute =
                                i64::from(OPEN_MINUTE).saturating_add(bucket.saturating_mul(width));
                            let timestamp = day
                                .saturating_mul(86_400_000_000)
                                .saturating_add(bucket_minute.saturating_mul(60_000_000))
                                .saturating_sub(indicators::IST_OFFSET_MICROS);
                            if timestamps.last().copied() != Some(timestamp) {
                                timestamps.push(timestamp);
                            }
                        }
                    }
                }
                DayKind::Closed => {}
                DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => {
                    panic!("fixture calendar day {day} is unmeasured")
                }
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("calendar receipt is constructible")
            .require_complete()
            .expect("fixture calendar is complete")
    }

    fn admission_policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(1),
            min_independent_sessions: Some(1),
            min_trades: Some(1),
            max_mae_paisa: Some(u64::MAX),
            min_worst_reward_risk_ppm: Some(0),
            min_win_rate_ppm: Some(0),
            min_wilson_win_rate_ppm: Some(0),
            min_return_drawdown_ppm: Some(0),
            min_weakest_period_return_paisa: Some(i64::MIN),
            max_pbo_ppm: Some(1_000_000),
            max_fwer_p_value_ppm: Some(1_000_000),
            max_spa_p_value_ppm: Some(1_000_000),
            min_decided_folds: Some(1),
            max_ambiguous_fill_rate_ppm: Some(1_000_000),
            max_gap_affected_rate_ppm: Some(1_000_000),
            max_session_concentration_ppm: Some(1_000_000),
            max_largest_trade_profit_share_ppm: Some(1_000_000),
            max_drawdown_paisa: Some(u64::MAX),
            max_worst_trade_loss_paisa: Some(u64::MAX),
            max_losing_trade_rate_ppm: Some(1_000_000),
            max_losing_trades: Some(u64::MAX),
            min_pessimistic_profit_paisa: Some(i64::MIN),
            min_winning_trades: Some(0),
            min_average_win_paisa: Some(0),
            max_average_loss_paisa: Some(u64::MAX),
            min_profit_factor_ppm: Some(0),
            max_consecutive_losing_streak: Some(u64::MAX),
            min_consecutive_winning_streak: Some(0),
            min_bootstrap_draws: Some(1),
            min_bootstrap_strategies: Some(1),
            min_bootstrap_periods: Some(1),
            min_pbo_contributing_folds: Some(1),
            max_pbo_unrankable_folds: Some(u64::MAX),
            min_profitable_oos_folds: Some(1),
            min_oos_pessimistic_return_paisa: Some(i64::MIN),
            max_white_reality_p_value_ppm: Some(1_000_000),
            max_romano_wolf_p_value_ppm: Some(1_000_000),
            require_white_reality_rejection: Some(false),
            require_romano_wolf_rejection: Some(false),
        })
        .expect("fully explicit relaxed admission policy")
    }

    fn noninvented_unmeasured_direct_evidence() -> AdmissionEvidenceV1 {
        AdmissionEvidenceV1::new(AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Unmeasured,
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Unmeasured,
            max_mae_paisa: ObservedU64V1::Measured(0),
            worst_reward_risk_ppm: ObservedU64V1::Unmeasured,
            win_rate_ppm: ObservedU64V1::Unmeasured,
            wilson_win_rate_ppm: ObservedU64V1::Unmeasured,
            return_drawdown_ppm: ObservedU64V1::Measured(0),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            pbo_ppm: ObservedU64V1::Unmeasured,
            fwer_p_value_ppm: ObservedU64V1::Measured(0),
            spa_p_value_ppm: ObservedU64V1::Measured(0),
            decided_folds: ObservedU64V1::Measured(10),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(0),
            gap_affected_rate_ppm: ObservedU64V1::Measured(0),
            session_concentration_ppm: ObservedU64V1::Measured(0),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(0),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Unmeasured,
            worst_trade_loss_paisa: ObservedU64V1::Unmeasured,
            losing_trade_rate_ppm: ObservedU64V1::Unmeasured,
            losing_trades: ObservedU64V1::Unmeasured,
            pessimistic_profit_paisa: ObservedI64V1::Unmeasured,
            winning_trades: ObservedU64V1::Unmeasured,
            average_win_paisa: ObservedU64V1::Unmeasured,
            average_loss_paisa: ObservedU64V1::Unmeasured,
            profit_factor_ppm: ObservedU64V1::Measured(0),
            consecutive_losing_streak: ObservedU64V1::Measured(0),
            consecutive_winning_streak: ObservedU64V1::Measured(0),
            bootstrap_draws: ObservedU64V1::Measured(1_000),
            bootstrap_strategies: ObservedU64V1::Measured(100),
            bootstrap_periods: ObservedU64V1::Measured(300),
            pbo_contributing_folds: ObservedU64V1::Unmeasured,
            pbo_unrankable_folds: ObservedU64V1::Unmeasured,
            profitable_oos_folds: ObservedU64V1::Measured(10),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(0),
            white_reality_p_value_ppm: ObservedU64V1::Measured(0),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(0),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        })
        .expect("explicit evidence with direct values left unmeasured")
    }

    fn resolved_grids<'a>(
        instrument: &'a InstrumentKey,
        bars: &'a [indicators::Candle],
    ) -> (
        ExecutionSeriesV1<'a>,
        ResolvedExitGridV1,
        ResolvedExitGridV1,
    ) {
        let series = ExecutionSeriesV1::new(
            instrument,
            TEST_FEED,
            TEST_COMMIT,
            TEST_CALENDAR_POLICY,
            bars,
        )
        .expect("attested one-minute execution fixture");
        let long = exit_policy(Side::Long)
            .resolve_attested(series)
            .expect("long resolution fixture");
        let short = exit_policy(Side::Short)
            .resolve_attested(series)
            .expect("short resolution fixture");
        (series, long, short)
    }

    fn identities(
        family: InstrumentFamilyV1,
        ranking_policy_digest: [u8; 32],
        bars: &[indicators::Candle],
        column: &Column,
        long: &ResolvedExitGridV1,
        short: &ResolvedExitGridV1,
        policy: &AdmissionPolicyV1,
    ) -> PopulationIdentitiesV2 {
        let evaluation_policy_digest = column
            .evaluation_spec_token()
            .map(|token| brutex_core::blake3::hash(token.fingerprint_v1().as_bytes()))
            .expect("ordinary column carries evaluator identity");
        PopulationIdentitiesV2 {
            run_identity: match family {
                InstrumentFamilyV1::Nifty => [0x11; 32],
                InstrumentFamilyV1::BankNifty => [0x12; 32],
            },
            data_digest: runner::identity::data_digest(bars),
            feed_digest: brutex_core::blake3::hash(TEST_FEED.as_bytes()),
            source_commit_digest: brutex_core::blake3::hash(TEST_COMMIT.as_bytes()),
            vocabulary_digest: [0x22; 32],
            evaluation_policy_digest,
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: long.policy_digest(),
                    resolved_digest: long.digest(),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: short.policy_digest(),
                    resolved_digest: short.digest(),
                },
            },
            admission_policy_digest: policy.digest(),
            ranking_policy_digest,
            calendar_policy_digest: TEST_CALENDAR_POLICY,
            daily_reference_policy_digest: [0x44; 32],
        }
    }

    fn execution_run(
        instrument: &InstrumentKey,
        bars: &[indicators::Candle],
        ladder: Ladder,
        mask_words: [u64; 6],
        direction: TradeDirectionV1,
    ) -> Result<ExecutionRunV1, String> {
        let mask = runner::replay_mask::from_stored_words(mask_words)
            .map_err(|why| format!("execution mask fixture refused: {why}"))?;
        let run = Run {
            mask,
            direction: match direction {
                TradeDirectionV1::Long => Direction::Long,
                TradeDirectionV1::Short => Direction::Short,
            },
            instrument,
            timeframe: "1min",
            params: Params::of(ladder),
            data_digest: runner::identity::data_digest(bars),
            commit: TEST_COMMIT,
            feed: TEST_FEED,
        };
        ExecutionRunV1::new(&run, bars, None)
            .map_err(|why| format!("execution run fixture refused: {why:?}"))
    }

    fn evaluated_grid(
        fixture: &DurablePopulationFixture,
        series: ExecutionSeriesV1<'_>,
        resolved: &ResolvedExitGridV1,
        mask_words: [u64; 6],
        direction: TradeDirectionV1,
    ) -> EvaluatedExitGridV1 {
        let run = execution_run(
            &fixture.instrument,
            &fixture.bars,
            fixture.ladder,
            mask_words,
            direction,
        )
        .expect("directional execution run");
        resolved
            .evaluate_training_grid_attested(series, &fixture.column, fixture.horizon, run)
            .expect("complete evaluated grid")
    }

    fn write_population(
        root: &Path,
        symbol: &str,
        family: InstrumentFamilyV1,
        rung_seconds: u32,
        ranking_policy_digest: [u8; 32],
    ) -> DurablePopulationFixture {
        let bars = runner::synthetic::sessions(8);
        let instrument = instrument(symbol);
        let mut evaluator = evaluator();
        let column = Column::build(&bars, &mut evaluator);
        let (series, long_grid, short_grid) = resolved_grids(&instrument, &bars);
        let policy = admission_policy();
        let horizon = Horizon::bars(2).expect("two-bar horizon");
        let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
        let authority = CompletePopulationAuthorityV1 {
            instrument_family: family,
            rung_seconds,
            horizon,
            requested_span: requested_span(),
            identities: identities(
                family,
                ranking_policy_digest,
                &bars,
                &column,
                &long_grid,
                &short_grid,
                &policy,
            ),
            signal_calendar: complete_calendar(rung_seconds),
            execution_calendar: complete_calendar(60),
            admission_policy: policy,
            long_exit_grid: &long_grid,
            short_exit_grid: &short_grid,
            execution_series: series,
            execution_column: &column,
        };
        let produced = produce_population_admission_v1(
            &runner::Sweeper::new(ladder),
            column.clone(),
            authority,
            &|_, _, _| {},
            |words, direction| execution_run(&instrument, &bars, ladder, words, direction),
            |context| {
                let assurance_bp = u64::try_from(context.cell.assurance_bp())
                    .map_err(|_| "negative assurance basis-point fixture".to_owned())?;
                let assurance_ppm = assurance_bp
                    .checked_mul(100)
                    .ok_or_else(|| "assurance ppm fixture overflowed".to_owned())?;
                Ok(PopulationCellEvidenceV1 {
                    assurance_ppm,
                    admission: noninvented_unmeasured_direct_evidence(),
                })
            },
        )
        .expect("complete uncapped population fixture");
        assert!(produced.population_run().is_complete());
        assert!(produced.population_run().closed > 0);
        let population_id = produced.population_id();
        let prepared = produced.into_prepared();
        let rows = prepared.rows().to_vec();
        let decisions = prepared.decisions().to_vec();
        let population_v4 = prepared.receipt();
        let population_v4_digest = population_v4
            .content_digest()
            .expect("Population V4 digest");
        PopulationLedger::open(root)
            .expect("open population ledger")
            .append_complete_v4(&rows, &population_v4)
            .expect("commit Population V4");
        let admission = AdmissionAuthorityLedger::open(root)
            .expect("open admission ledger")
            .commit(
                population_id,
                population_v4_digest,
                &prepared.policy(),
                &decisions,
            )
            .expect("commit admission authority")
            .receipt()
            .clone();
        DurablePopulationFixture {
            population_id,
            rows,
            decisions,
            population_v4,
            admission,
            instrument,
            bars,
            column,
            long_grid,
            short_grid,
            ladder,
            horizon,
        }
    }

    fn chosen_from_row(row: &PopulationRowV1) -> Chosen {
        Chosen {
            stop: row
                .exit
                .stop
                .map(|value| usize::try_from(value).expect("stop fits usize")),
            target: row
                .exit
                .target
                .map(|value| usize::try_from(value).expect("target fits usize")),
            tsl: row
                .exit
                .tsl
                .map(|value| usize::try_from(value).expect("TSL fits usize")),
            ttp: row.exit.ttp.map(|(arm, trail)| Ttp {
                arm: usize::try_from(arm).expect("TTP arm fits usize"),
                trail: usize::try_from(trail).expect("TTP trail fits usize"),
            }),
        }
    }

    fn write_execution(root: &Path, fixture: &DurablePopulationFixture) {
        let fingerprint = fixture
            .column
            .evaluation_spec_token()
            .expect("fixture evaluator identity")
            .fingerprint_v1();
        let long_parameters = ExecutionParametersV1::from_resolved(
            fixture.population_v4,
            TradeDirectionV1::Long,
            fixture.horizon,
            Params::of(fixture.ladder),
            fingerprint,
            &fixture.long_grid,
        )
        .expect("long execution parameters");
        let short_parameters = ExecutionParametersV1::from_resolved(
            fixture.population_v4,
            TradeDirectionV1::Short,
            fixture.horizon,
            Params::of(fixture.ladder),
            fingerprint,
            &fixture.short_grid,
        )
        .expect("short execution parameters");
        let series = ExecutionSeriesV1::new(
            &fixture.instrument,
            TEST_FEED,
            TEST_COMMIT,
            TEST_CALENDAR_POLICY,
            &fixture.bars,
        )
        .expect("reopened execution series");
        let mut dispositions = Vec::with_capacity(fixture.rows.len());
        let long_count =
            usize::try_from(fixture.long_grid.cell_count()).expect("long cell count fits usize");
        let short_count =
            usize::try_from(fixture.short_grid.cell_count()).expect("short cell count fits usize");
        let per_mask = long_count
            .checked_add(short_count)
            .expect("per-mask cell count fits usize");
        assert_eq!(fixture.rows.len() % per_mask, 0);
        for mask_start in (0..fixture.rows.len()).step_by(per_mask) {
            let mask_words = fixture
                .rows
                .get(mask_start)
                .expect("canonical mask block starts with one row")
                .mask_words;
            for (side_offset, count, direction, resolved, parameters) in [
                (
                    0_usize,
                    long_count,
                    TradeDirectionV1::Long,
                    &fixture.long_grid,
                    &long_parameters,
                ),
                (
                    long_count,
                    short_count,
                    TradeDirectionV1::Short,
                    &fixture.short_grid,
                    &short_parameters,
                ),
            ] {
                let evaluated = evaluated_grid(fixture, series, resolved, mask_words, direction);
                let validated = resolved
                    .validate_evaluation(&evaluated)
                    .expect("complete evaluation validates once");
                for ordinal in 0..count {
                    let index = mask_start
                        .checked_add(side_offset)
                        .and_then(|value| value.checked_add(ordinal))
                        .expect("fixture row index fits usize");
                    let row = *fixture.rows.get(index).expect("canonical population row");
                    let decision = fixture
                        .decisions
                        .get(index)
                        .expect("canonical admission decision");
                    assert_eq!(row.direction, direction);
                    assert_eq!(row.mask_words, mask_words);
                    let coordinate = chosen_from_row(&row);
                    assert_eq!(
                        validated
                            .cell(ordinal)
                            .map(Chosen::from_cell)
                            .expect("canonical evaluated cell"),
                        coordinate
                    );
                    let classification = resolved
                        .classify_coordinate(&validated, coordinate)
                        .expect("canonical coordinate classifies terminally");
                    let capability = classification
                        .selected()
                        .map(|selected| {
                            ExecutionStrategyCapabilityV1::new(parameters, row, selected)
                        })
                        .transpose()
                        .expect("authorized sparse capability");
                    dispositions.push(
                        ExecutionRowDispositionV2::from_classification(
                            &fixture.population_v4,
                            &fixture.admission,
                            row,
                            decision,
                            parameters,
                            &classification,
                            capability,
                        )
                        .expect("durable execution disposition"),
                    );
                }
            }
        }
        assert_eq!(dispositions.len(), fixture.rows.len());
        let prepared = {
            let mut populations = PopulationLedger::open(root).expect("reopen populations");
            let mut admissions = AdmissionAuthorityLedger::open(root).expect("reopen admissions");
            PreparedExecutionDispositionsV2::from_ledgers(
                &mut populations,
                &mut admissions,
                fixture.population_id,
                long_parameters,
                short_parameters,
                dispositions,
            )
            .expect("reconcile real Population V4/admission/execution rows")
        };
        ExecutionDispositionLedgerV2::open(root)
            .expect("open execution V2 ledger")
            .append_complete(&prepared)
            .expect("commit execution V2 authority");
    }

    fn completion_record(
        root: &Path,
        population_id: [u8; 32],
    ) -> (u64, [u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2]) {
        let path = ExecutionDispositionLedgerV2::completion_path(root);
        let mut bytes = Vec::new();
        File::open(&path)
            .expect("open completion file")
            .read_to_end(&mut bytes)
            .expect("read completion file");
        let records = bytes
            .get(usize::try_from(HEADER_BYTES).expect("header fits usize")..)
            .expect("completion header exists");
        for (index, record) in records
            .chunks_exact(EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2)
            .enumerate()
        {
            if record.get(COMPLETION_POPULATION_OFFSET..COMPLETION_POPULATION_OFFSET + 32)
                == Some(population_id.as_slice())
            {
                let record: [u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2] =
                    record.try_into().expect("fixed completion record");
                let byte_offset = HEADER_BYTES
                    .checked_add(
                        u64::try_from(index)
                            .expect("record index fits u64")
                            .checked_mul(
                                u64::try_from(EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2)
                                    .expect("stride fits u64"),
                            )
                            .expect("record byte offset fits u64"),
                    )
                    .expect("absolute completion offset fits u64");
                return (byte_offset, record);
            }
        }
        panic!("population completion fixture was not found")
    }

    fn row_record(
        root: &Path,
        population_id: [u8; 32],
    ) -> (u64, [u8; EXECUTION_DISPOSITION_ROW_STRIDE_V2]) {
        let path = ExecutionDispositionLedgerV2::row_path(root);
        let mut bytes = Vec::new();
        File::open(&path)
            .expect("open disposition row file")
            .read_to_end(&mut bytes)
            .expect("read disposition row file");
        let records = bytes
            .get(usize::try_from(HEADER_BYTES).expect("header fits usize")..)
            .expect("row header exists");
        for (index, record) in records
            .chunks_exact(EXECUTION_DISPOSITION_ROW_STRIDE_V2)
            .enumerate()
        {
            if record.get(ROW_POPULATION_OFFSET..ROW_POPULATION_OFFSET + 32)
                == Some(population_id.as_slice())
            {
                let record: [u8; EXECUTION_DISPOSITION_ROW_STRIDE_V2] =
                    record.try_into().expect("fixed row record");
                let byte_offset = HEADER_BYTES
                    .checked_add(
                        u64::try_from(index)
                            .expect("record index fits u64")
                            .checked_mul(
                                u64::try_from(EXECUTION_DISPOSITION_ROW_STRIDE_V2)
                                    .expect("row stride fits u64"),
                            )
                            .expect("row byte offset fits u64"),
                    )
                    .expect("absolute row offset fits u64");
                return (byte_offset, record);
            }
        }
        panic!("population row fixture was not found")
    }

    fn reseal_completion(raw: &mut [u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2]) {
        let payload: [u8; COMPLETION_PAYLOAD_BYTES] = raw[..COMPLETION_PAYLOAD_BYTES]
            .try_into()
            .expect("completion payload");
        raw[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&brutex_core::blake3::hash(&payload));
    }

    fn write_exact_record(path: &Path, offset: u64, raw: &[u8]) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open exact mutation target");
        file.seek(SeekFrom::Start(offset))
            .expect("seek exact mutation target");
        file.write_all(raw).expect("write exact mutation bytes");
        file.sync_all().expect("sync exact mutation bytes");
    }

    #[test]
    fn real_three_ledger_pair_is_exact_and_every_authority_drift_refuses() {
        let root = store_root("real-three-ledger-pair");
        let ranking = [0x33; 32];
        let nifty = write_population(&root, "NIFTY", InstrumentFamilyV1::Nifty, 60, ranking);
        let bank_nifty = write_population(
            &root,
            "BANKNIFTY",
            InstrumentFamilyV1::BankNifty,
            60,
            ranking,
        );

        write_execution(&root, &nifty);
        let missing = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            nifty.population_id,
            bank_nifty.population_id,
        )
        .expect_err("a population without Execution V2 completion must refuse");
        assert!(missing.contains("no complete Execution V2 authority"));

        write_execution(&root, &bank_nifty);
        let (mut nifty_view, bank_view) = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            nifty.population_id,
            bank_nifty.population_id,
        )
        .expect("exact completed NIFTY/BANKNIFTY pair opens");
        let snapshot = nifty_view.snapshot().clone();
        assert_eq!(snapshot.population.population_id, nifty.population_id);
        assert_eq!(snapshot.population.family, InstrumentFamilyV1::Nifty);
        assert_eq!(
            snapshot.population.row_count,
            u64::try_from(nifty.rows.len()).expect("row count fits u64")
        );
        assert_eq!(
            snapshot.admission.decision_count,
            snapshot.execution.disposition_count
        );
        let matrix_total = snapshot
            .execution
            .admission_execution_counts
            .into_iter()
            .flatten()
            .try_fold(0_u64, u64::checked_add)
            .expect("small real matrix total");
        assert_eq!(matrix_total, snapshot.population.row_count);
        let limit = snapshot.population.row_count.min(256);
        let page = nifty_view
            .page(0, limit)
            .expect("real triple-authority page joins");
        assert_eq!(page.total, snapshot.population.row_count);
        assert_eq!(page.rows.len(), usize::try_from(limit).expect("limit fits"));
        for (sequence, row) in page.rows.iter().enumerate() {
            assert_eq!(row.population.population_id, nifty.population_id);
            assert_eq!(
                row.population.sequence,
                u64::try_from(sequence).expect("sequence fits")
            );
            assert_eq!(
                row.population.strategy_digest,
                row.admission.strategy_digest
            );
            assert_eq!(
                row.population.strategy_digest,
                row.execution.strategy_digest
            );
            assert_eq!(
                row.population.payload_digest().expect("row payload"),
                row.admission.population_payload_digest
            );
            assert_eq!(
                row.admission.population_payload_digest,
                row.execution.population_payload_digest
            );
        }
        drop(bank_view);
        drop(nifty_view);

        let family = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            bank_nifty.population_id,
            nifty.population_id,
        )
        .expect_err("swapped family identities must refuse");
        assert!(family.contains("expected Nifty"));

        let foreign_cohort = write_population(
            &root,
            "BANKNIFTY",
            InstrumentFamilyV1::BankNifty,
            60,
            [0x93; 32],
        );
        write_execution(&root, &foreign_cohort);
        assert!(
            SelectionAuthorityLedgerV4::open_pair_read(
                &root,
                nifty.population_id,
                foreign_cohort.population_id,
            )
            .is_err(),
            "different ranking/cohort authority must refuse"
        );

        let foreign_rung = write_population(
            &root,
            "BANKNIFTY",
            InstrumentFamilyV1::BankNifty,
            120,
            ranking,
        );
        write_execution(&root, &foreign_rung);
        let rung = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            nifty.population_id,
            foreign_rung.population_id,
        )
        .expect_err("different signal rungs must refuse");
        assert!(rung.contains("different timeframes"));

        let completion_path = ExecutionDispositionLedgerV2::completion_path(&root);
        let (completion_offset, original_completion) =
            completion_record(&root, bank_nifty.population_id);

        let mut foreign_completion = original_completion;
        foreign_completion[COMPLETION_POPULATION_V4_OFFSET] ^= 0x01;
        reseal_completion(&mut foreign_completion);
        write_exact_record(&completion_path, completion_offset, &foreign_completion);
        assert!(
            SelectionAuthorityLedgerV4::open_pair_read(
                &root,
                nifty.population_id,
                bank_nifty.population_id,
            )
            .is_err(),
            "foreign Population V4 completion binding must refuse"
        );
        write_exact_record(&completion_path, completion_offset, &original_completion);

        let mut matrix_corruption = original_completion;
        let matrix_cell: [u8; 8] = matrix_corruption
            [COMPLETION_MATRIX_OFFSET..COMPLETION_MATRIX_OFFSET + 8]
            .try_into()
            .expect("first matrix cell");
        let changed_matrix_cell = u64::from_le_bytes(matrix_cell)
            .checked_add(1)
            .expect("fixture matrix count can increase once");
        matrix_corruption[COMPLETION_MATRIX_OFFSET..COMPLETION_MATRIX_OFFSET + 8]
            .copy_from_slice(&changed_matrix_cell.to_le_bytes());
        reseal_completion(&mut matrix_corruption);
        write_exact_record(&completion_path, completion_offset, &matrix_corruption);
        let matrix = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            nifty.population_id,
            bank_nifty.population_id,
        )
        .expect_err("resealed matrix corruption must refuse");
        assert!(matrix.contains("matrix"));
        write_exact_record(&completion_path, completion_offset, &original_completion);

        let (mut stale_nifty, stale_bank) = SelectionAuthorityLedgerV4::open_pair_read(
            &root,
            nifty.population_id,
            bank_nifty.population_id,
        )
        .expect("restored exact pair reopens");
        let row_path = ExecutionDispositionLedgerV2::row_path(&root);
        let (row_offset, original_row) = row_record(&root, nifty.population_id);
        let mut same_length_mutation = original_row;
        same_length_mutation[0] ^= 0x80;
        write_exact_record(&row_path, row_offset, &same_length_mutation);
        let stale = stale_nifty
            .page(0, 1)
            .expect_err("post-open same-length row mutation must refuse");
        assert!(stale.contains("changed"));
        write_exact_record(&row_path, row_offset, &original_row);
        drop(stale_bank);
        drop(stale_nifty);

        clean_store(&root);
    }

    #[test]
    fn page_ceiling_is_exact_and_zero_is_a_valid_empty_probe() {
        assert!(require_page_limit(0).is_ok());
        assert!(require_page_limit(256).is_ok());
        assert!(require_page_limit(257).is_err());
    }

    #[test]
    fn execution_domain_maps_exactly_two_terminal_values() {
        assert_eq!(
            map_execution_tag(ExecutionDispositionTagV2::Authorized),
            ExecutionSelectionStatusV2::Authorized
        );
        assert_eq!(
            map_execution_tag(ExecutionDispositionTagV2::PolicyRefused),
            ExecutionSelectionStatusV2::PolicyRefused
        );
    }

    #[test]
    fn row_major_matrix_shape_is_not_caller_reorderable() {
        assert_eq!(
            matrix_rows([0, 1, 2, 3, 4, 5, 6, 7]),
            [[0, 1], [2, 3], [4, 5], [6, 7]]
        );
    }

    #[test]
    fn every_row_binding_term_is_fail_closed() {
        let expected = binding();
        assert!(require_row_binding(7, expected, expected, expected).is_ok());
        assert!(require_row_binding(8, expected, expected, expected).is_err());

        let mut variants = Vec::new();
        let mut changed = expected;
        changed.population_id = digest(9);
        variants.push(changed);
        changed = expected;
        changed.sequence = 8;
        variants.push(changed);
        changed = expected;
        changed.strategy_digest = digest(9);
        variants.push(changed);
        changed = expected;
        changed.payload_digest = digest(9);
        variants.push(changed);
        changed = expected;
        changed.population_v4_digest = digest(9);
        variants.push(changed);
        changed = expected;
        changed.admission_completion_digest = digest(9);
        variants.push(changed);
        changed = expected;
        changed.admission_status = AdmissionStatusV1::Rejected;
        variants.push(changed);

        for variant in variants {
            assert!(require_row_binding(7, expected, variant, expected).is_err());
            assert!(require_row_binding(7, expected, expected, variant).is_err());
        }
    }

    #[test]
    fn matrix_sum_refuses_overflow() {
        assert_eq!(checked_sum([1, 2, 3], "fixture"), Ok(6));
        assert!(checked_sum([u64::MAX, 1], "fixture").is_err());
    }

    #[test]
    fn independently_opened_view_locks_do_not_unlock_each_other() {
        let path = std::env::temp_dir().join(format!(
            "brutex-selection-v4-authority-lock-{}-{}",
            std::process::id(),
            NEXT_LOCK.fetch_add(1, Ordering::Relaxed)
        ));
        let created = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create lock fixture");
        drop(created);
        let first = independent_lock_handle(&path).expect("first independent view lock");
        let second = independent_lock_handle(&path).expect("second independent view lock");
        let writer = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("writer lock handle");

        first.lock_shared().expect("first shared lock");
        second.lock_shared().expect("second shared lock");
        assert!(writer.try_lock().is_err());
        first.unlock().expect("release first shared lock");
        assert!(writer.try_lock().is_err());
        second.unlock().expect("release second shared lock");
        writer
            .try_lock()
            .expect("exclusive lock after both readers");
        writer.unlock().expect("release exclusive lock");

        drop(File::open(&path).expect("fixture still exists"));
        std::fs::remove_file(path).expect("remove lock fixture");
    }
}
