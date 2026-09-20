//! Fail-closed Step-3 authority comparison.
//!
//! This module reads only the current authority formats: Population V4,
//! admission V1, Execution V2, Selection V4, stored-data completeness V1 and
//! Global Replay V2.  It never consults a V1/V2/V3 selection as a fallback and
//! never manufactures an absent receipt.  The result is a deterministic table
//! whose four states deliberately keep absence, dependency blockage and an
//! invalid authority separate.
//!
//! Opening these ledgers is proportional to their bounded file sizes.  The
//! comparison and table rendering are proportional to the fixed 16-population,
//! eight-selection authority surface and to the rendered identity bytes.  No
//! O(1) claim is made for opening or rendering the report.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use crate::admission_store::AdmissionAuthorityLedger;
use crate::execution_disposition_v2::{AdmissionExecutionMatrixV2, ExecutionDispositionLedgerV2};
use crate::global_replay_v2::{
    GLOBAL_REPLAY_V2_RUNGS_SECONDS, GlobalReplayLedgerV2, GlobalReplayManifestV2,
};
use crate::population::{InstrumentFamilyV1, PopulationLedger};
use crate::selection_v4::{PopulationReferenceV4, SelectionLedgerV4, SelectionReceiptV4};
use crate::stored_data_completeness::StoredDataCompletenessLedgerV1;
use runner::portfolio::Counters;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const POPULATION_COUNT: usize = 16;
const SELECTION_COUNT: usize = 8;
const POPULATIONS_PER_SELECTION: usize = 2;
const TOP_TEN_PER_SELECTION: u64 = 10;
const TOP_TWENTY_FIVE_PER_SELECTION: u64 = 25;
const GLOBAL_STREAM_COUNT: u64 = 200;

/// One of the six durable Step-3 stages, in dependency order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step3StageV1 {
    /// Calendar-bound Population V4 receipts.
    PopulationV4,
    /// Receipt-last institutional-admission decisions.
    Admission,
    /// Receipt-last exact Execution V2 dispositions.
    ExecutionV2,
    /// Exact stored signal/minute/daily-reference reconciliation.
    StoredData,
    /// Eight cohort-bound Selection V4 Top-N receipts.
    SelectionV4,
    /// The 8 × 25 Global Replay V2 execution-only authority.
    GlobalReplayV2,
}

impl Step3StageV1 {
    const fn label(self) -> &'static str {
        match self {
            Self::PopulationV4 => "Population V4",
            Self::Admission => "Admission V1",
            Self::ExecutionV2 => "Execution V2",
            Self::StoredData => "Stored data V1",
            Self::SelectionV4 => "Selection V4",
            Self::GlobalReplayV2 => "Global Replay V2",
        }
    }
}

/// Truth state of one comparison row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step3StatusV1 {
    /// The exact current-format authority exists and reconciles.
    Ready,
    /// Durable bytes are valid, but an upstream authority or full topology is absent.
    Blocked,
    /// The requested current-format receipt or file has not been measured/stored.
    Unmeasured,
    /// Existing bytes are corrupt, stale, foreign or internally inconsistent.
    Refused,
}

impl Step3StatusV1 {
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Blocked => "BLOCKED",
            Self::Unmeasured => "UNMEASURED",
            Self::Refused => "REFUSED",
        }
    }
}

/// One exact named count rendered in a comparison row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step3NamedCountV1 {
    name: &'static str,
    value: u64,
}

impl Step3NamedCountV1 {
    const fn new(name: &'static str, value: u64) -> Self {
        Self { name, value }
    }

    /// Stable count name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Exact observed value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
    }
}

/// One human-readable row backed by exact typed authority observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step3ComparisonRowV1 {
    stage: Step3StageV1,
    status: Step3StatusV1,
    identities: Vec<String>,
    counts: Vec<Step3NamedCountV1>,
    detail: String,
}

impl Step3ComparisonRowV1 {
    /// Durable stage represented by this row.
    #[must_use]
    pub const fn stage(&self) -> Step3StageV1 {
        self.stage
    }

    /// Truth state after direct and cross-authority validation.
    #[must_use]
    pub const fn status(&self) -> Step3StatusV1 {
        self.status
    }

    /// Full, untruncated hexadecimal identities carried by this stage.
    #[must_use]
    pub fn identities(&self) -> &[String] {
        &self.identities
    }

    /// Exact named counters carried by this stage.
    #[must_use]
    pub fn counts(&self) -> &[Step3NamedCountV1] {
        &self.counts
    }

    /// Human-readable reason or reconciliation statement.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Complete six-row comparison read model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step3ComparisonV1 {
    rows: Vec<Step3ComparisonRowV1>,
}

impl Step3ComparisonV1 {
    /// Rows in fixed dependency order.
    #[must_use]
    pub fn rows(&self) -> &[Step3ComparisonRowV1] {
        &self.rows
    }

    /// Whether every declared Step-3 authority is present and reconciled.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.rows
            .iter()
            .all(|row| row.status == Step3StatusV1::Ready)
    }

    /// Deterministic `CommonMark` table with exact, untruncated identities.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut rendered = String::from(
            "| Step-3 authority | State | Exact identities | Exact counts | Meaning |\n\
             |---|---|---|---|---|\n",
        );
        for row in &self.rows {
            let identities = if row.identities.is_empty() {
                "none".to_owned()
            } else {
                row.identities.join("<br>")
            };
            let counts = row
                .counts
                .iter()
                .map(|count| format!("{}={}", count.name, count.value))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = writeln!(
                rendered,
                "| {} | {} | {} | {} | {} |",
                row.stage.label(),
                row.status.label(),
                escape_cell(&identities),
                escape_cell(&counts),
                escape_cell(&row.detail),
            );
        }
        rendered
    }
}

/// Explicit ceilings for every scan performed by the read model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_field_names,
    reason = "each independent ceiling is explicitly named as a maximum"
)]
pub struct Step3ReadBoundsV1 {
    max_file_bytes: u64,
    max_selection_receipts: usize,
    max_stored_receipts: usize,
    max_replay_completions: usize,
}

impl Step3ReadBoundsV1 {
    /// Constructs nonzero bounds large enough to hold the required authority surface.
    ///
    /// # Errors
    ///
    /// Refuses a zero byte bound or a record ceiling smaller than 8, 16 or 1.
    pub fn new(
        max_file_bytes: u64,
        max_selection_receipts: usize,
        max_stored_receipts: usize,
        max_replay_completions: usize,
    ) -> Result<Self, String> {
        if max_file_bytes == 0 {
            return Err("Step-3 max_file_bytes must be greater than zero".to_owned());
        }
        if max_selection_receipts < SELECTION_COUNT {
            return Err("Step-3 selection bound is smaller than eight receipts".to_owned());
        }
        if max_stored_receipts < POPULATION_COUNT {
            return Err("Step-3 stored-data bound is smaller than sixteen receipts".to_owned());
        }
        if max_replay_completions == 0 {
            return Err("Step-3 replay-completion bound must be greater than zero".to_owned());
        }
        Ok(Self {
            max_file_bytes,
            max_selection_receipts,
            max_stored_receipts,
            max_replay_completions,
        })
    }
}

/// Exact IDs the caller wants compared; no discovery fallback is permitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step3AuthorityRequestV1 {
    population_ids: [[u8; 32]; POPULATION_COUNT],
    selection_ids: [[u8; 32]; SELECTION_COUNT],
    replay_completion_id: [u8; 32],
}

impl Step3AuthorityRequestV1 {
    /// Constructs an exact request in canonical rung then `[NIFTY, BANKNIFTY]` order.
    ///
    /// # Errors
    ///
    /// Refuses zero or duplicated IDs.  IDs are never inferred from another stage.
    pub fn new(
        population_ids: [[u8; 32]; POPULATION_COUNT],
        selection_ids: [[u8; 32]; SELECTION_COUNT],
        replay_completion_id: [u8; 32],
    ) -> Result<Self, String> {
        validate_unique_ids("Population V4", &population_ids)?;
        validate_unique_ids("Selection V4", &selection_ids)?;
        require_id("Global Replay V2 completion", replay_completion_id)?;
        Ok(Self {
            population_ids,
            selection_ids,
            replay_completion_id,
        })
    }

    /// Sixteen Population V4 IDs in canonical rung/family order.
    #[must_use]
    pub const fn population_ids(&self) -> &[[u8; 32]; POPULATION_COUNT] {
        &self.population_ids
    }

    /// Eight Selection V4 IDs in canonical rung order.
    #[must_use]
    pub const fn selection_ids(&self) -> &[[u8; 32]; SELECTION_COUNT] {
        &self.selection_ids
    }

    /// One exact Global Replay V2 completion ID.
    #[must_use]
    pub const fn replay_completion_id(&self) -> [u8; 32] {
        self.replay_completion_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopulationFact {
    population_id: [u8; 32],
    completion_digest: [u8; 32],
    ordered_row_digest: [u8; 32],
    row_count: u64,
    rung_seconds: u32,
    family: InstrumentFamilyV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdmissionFact {
    population_id: [u8; 32],
    population_digest: [u8; 32],
    completion_digest: [u8; 32],
    decision_count: u64,
    admitted: u64,
    rejected: u64,
    unmeasured: u64,
    refused: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExecutionFact {
    population_id: [u8; 32],
    population_digest: [u8; 32],
    admission_digest: [u8; 32],
    completion_id: [u8; 32],
    parameter_ids: [[u8; 32]; 2],
    row_count: u64,
    authorized: u64,
    policy_refused: u64,
    matrix: AdmissionExecutionMatrixV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredFact {
    population_id: [u8; 32],
    population_digest: [u8; 32],
    receipt_digest: [u8; 32],
    data_digest: [u8; 32],
    signal_records: u64,
    minute_context_records: u64,
    execution_records: u64,
    daily_records: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectionFact {
    selection_id: [u8; 32],
    rung_seconds: u32,
    references: [PopulationReferenceV4; POPULATIONS_PER_SELECTION],
    top_ten: u64,
    top_twenty_five: u64,
}

#[derive(Debug)]
struct StageProbe<T> {
    status: Step3StatusV1,
    evidence: Option<T>,
    identities: Vec<String>,
    counts: Vec<Step3NamedCountV1>,
    detail: String,
}

impl<T> StageProbe<T> {
    fn ready(
        evidence: T,
        identities: Vec<String>,
        counts: Vec<Step3NamedCountV1>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status: Step3StatusV1::Ready,
            evidence: Some(evidence),
            identities,
            counts,
            detail: detail.into(),
        }
    }

    fn unmeasured(identities: Vec<String>, detail: impl Into<String>) -> Self {
        Self {
            status: Step3StatusV1::Unmeasured,
            evidence: None,
            identities,
            counts: Vec::new(),
            detail: detail.into(),
        }
    }

    fn refused(identities: Vec<String>, detail: impl Into<String>) -> Self {
        Self {
            status: Step3StatusV1::Refused,
            evidence: None,
            identities,
            counts: Vec::new(),
            detail: detail.into(),
        }
    }

    fn refuse(&mut self, detail: impl Into<String>) {
        self.status = Step3StatusV1::Refused;
        self.evidence = None;
        self.detail = detail.into();
    }

    fn block(&mut self, detail: impl Into<String>) {
        self.status = Step3StatusV1::Blocked;
        self.evidence = None;
        self.detail = detail.into();
    }

    fn is_ready(&self) -> bool {
        self.status == Step3StatusV1::Ready
    }

    fn row(&self, stage: Step3StageV1) -> Step3ComparisonRowV1 {
        Step3ComparisonRowV1 {
            stage,
            status: self.status,
            identities: self.identities.clone(),
            counts: self.counts.clone(),
            detail: self.detail.clone(),
        }
    }
}

/// Opens and compares the complete current-format Step-3 authority chain.
///
/// A missing primary stage file or requested receipt is `UNMEASURED`. Existing
/// bytes that fail validation or name a foreign authority are `REFUSED`. A
/// valid stage whose prerequisite is not ready, or a Selection V4 set shorter
/// than the complete 8 × 25 topology, is `BLOCKED`.
///
/// # Errors
///
/// This function returns an error only for an invalid caller-supplied bound or
/// request. Durable stage failures are represented in their table row so one
/// bad stage cannot hide the state of the other five.
pub fn compare_step3_on_disk_v1(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> Result<Step3ComparisonV1, String> {
    validate_runtime_request(request, bounds)?;

    let population = read_populations(root, request, bounds);
    let mut admission = read_admissions(root, request, bounds);
    reconcile_admission(&population, &mut admission);
    let mut execution = read_execution(root, request, bounds);
    reconcile_execution(&population, &admission, &mut execution);
    let mut stored = read_stored(root, request, bounds);
    reconcile_stored(&population, &mut stored);
    let mut selection = read_selections(root, request, bounds);
    reconcile_selections(&population, &admission, &execution, &mut selection);
    let mut replay = read_global_replay(root, request, bounds);
    reconcile_global_replay(
        request,
        &population,
        &admission,
        &execution,
        &selection,
        &mut replay,
    );

    Ok(Step3ComparisonV1 {
        rows: vec![
            population.row(Step3StageV1::PopulationV4),
            admission.row(Step3StageV1::Admission),
            execution.row(Step3StageV1::ExecutionV2),
            stored.row(Step3StageV1::StoredData),
            selection.row(Step3StageV1::SelectionV4),
            replay.row(Step3StageV1::GlobalReplayV2),
        ],
    })
}

fn validate_runtime_request(
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> Result<(), String> {
    validate_unique_ids("Population V4", &request.population_ids)?;
    validate_unique_ids("Selection V4", &request.selection_ids)?;
    require_id("Global Replay V2 completion", request.replay_completion_id)?;
    Step3ReadBoundsV1::new(
        bounds.max_file_bytes,
        bounds.max_selection_receipts,
        bounds.max_stored_receipts,
        bounds.max_replay_completions,
    )?;
    Ok(())
}

fn read_populations(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<Vec<PopulationFact>> {
    let expected = labeled_ids("population", &request.population_ids);
    let primary = PopulationLedger::receipt_v4_path(root);
    if !primary.exists() {
        return StageProbe::unmeasured(expected, format!("{} is absent", primary.display()));
    }
    let ledger = match PopulationLedger::open_read_bounded(root, bounds.max_file_bytes) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(expected, why),
    };
    let mut facts = Vec::with_capacity(POPULATION_COUNT);
    let mut identities = Vec::with_capacity(POPULATION_COUNT * 2);
    for (index, population_id) in request.population_ids.iter().copied().enumerate() {
        let Some(receipt) = ledger.receipt_v4(&population_id) else {
            return StageProbe::unmeasured(
                identities,
                format!(
                    "Population V4 receipt {} is absent after {} requested receipts",
                    hex(&population_id),
                    facts.len()
                ),
            );
        };
        let completion_digest = match receipt.content_digest() {
            Ok(value) => value,
            Err(why) => return StageProbe::refused(identities, why),
        };
        let v2 = receipt.v3().v2();
        let Some(expected_rung) = GLOBAL_REPLAY_V2_RUNGS_SECONDS.get(index / 2).copied() else {
            return StageProbe::refused(
                identities,
                format!("Population V4 canonical rung slot {index} is absent"),
            );
        };
        let expected_family = expected_family(index);
        if v2.rung_seconds != expected_rung || v2.instrument_family != expected_family {
            return StageProbe::refused(
                identities,
                format!(
                    "Population V4 slot {index} expected {expected_family:?}/{expected_rung}s, found {:?}/{}s",
                    v2.instrument_family, v2.rung_seconds
                ),
            );
        }
        identities.push(labeled_id("population", population_id));
        identities.push(labeled_id("completion", completion_digest));
        facts.push(PopulationFact {
            population_id,
            completion_digest,
            ordered_row_digest: v2.ordered_row_digest,
            row_count: v2.row_count,
            rung_seconds: v2.rung_seconds,
            family: v2.instrument_family,
        });
    }
    let rows = match sum_u64(facts.iter().map(|fact| fact.row_count), "population rows") {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(identities, why),
    };
    StageProbe::ready(
        facts,
        identities,
        vec![
            Step3NamedCountV1::new("receipts", POPULATION_COUNT as u64),
            Step3NamedCountV1::new("rows", rows),
        ],
        "all 16 calendar-bound Population V4 receipts match canonical rung/family slots",
    )
}

fn read_admissions(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<Vec<AdmissionFact>> {
    let expected = labeled_ids("population", &request.population_ids);
    let primary = AdmissionAuthorityLedger::completion_path(root);
    if !primary.exists() {
        return StageProbe::unmeasured(expected, format!("{} is absent", primary.display()));
    }
    let ledger = match AdmissionAuthorityLedger::open_read_bounded(root, bounds.max_file_bytes) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(expected, why),
    };
    let mut facts = Vec::with_capacity(POPULATION_COUNT);
    let mut identities = Vec::with_capacity(POPULATION_COUNT * 2);
    for population_id in request.population_ids {
        let completion = match ledger.completion(population_id) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return StageProbe::unmeasured(
                    identities,
                    format!("admission completion for {} is absent", hex(&population_id)),
                );
            }
            Err(why) => return StageProbe::refused(identities, why),
        };
        let completion_digest = match completion.digest() {
            Ok(value) => value,
            Err(why) => return StageProbe::refused(identities, why),
        };
        identities.push(labeled_id("population", population_id));
        identities.push(labeled_id("completion", completion_digest));
        facts.push(AdmissionFact {
            population_id,
            population_digest: completion.population_v4_completion_digest(),
            completion_digest,
            decision_count: completion.decision_count(),
            admitted: completion.admitted_count(),
            rejected: completion.rejected_count(),
            unmeasured: completion.unmeasured_count(),
            refused: completion.refused_count(),
        });
    }
    admission_probe(facts, identities)
}

fn admission_probe(
    facts: Vec<AdmissionFact>,
    identities: Vec<String>,
) -> StageProbe<Vec<AdmissionFact>> {
    let decisions = match sum_u64(
        facts.iter().map(|fact| fact.decision_count),
        "admission rows",
    ) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(identities, why),
    };
    let admitted = sum_or_refuse(&facts, &identities, |fact| fact.admitted, "admitted rows");
    let rejected = sum_or_refuse(&facts, &identities, |fact| fact.rejected, "rejected rows");
    let unmeasured = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.unmeasured,
        "unmeasured rows",
    );
    let refused = sum_or_refuse(&facts, &identities, |fact| fact.refused, "refused rows");
    let (Ok(admitted), Ok(rejected), Ok(unmeasured), Ok(refused)) =
        (admitted, rejected, unmeasured, refused)
    else {
        return StageProbe::refused(identities, "admission aggregate count overflowed");
    };
    StageProbe::ready(
        facts,
        identities,
        vec![
            Step3NamedCountV1::new("receipts", POPULATION_COUNT as u64),
            Step3NamedCountV1::new("decisions", decisions),
            Step3NamedCountV1::new("admitted", admitted),
            Step3NamedCountV1::new("rejected", rejected),
            Step3NamedCountV1::new("unmeasured", unmeasured),
            Step3NamedCountV1::new("refused", refused),
        ],
        "all 16 receipt-last admission completions reopened without generation drift",
    )
}

fn read_execution(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<Vec<ExecutionFact>> {
    let expected = labeled_ids("population", &request.population_ids);
    let paths = execution_paths(root);
    if !ExecutionDispositionLedgerV2::completion_path(root).exists() {
        return StageProbe::unmeasured(
            expected,
            format!(
                "{} is absent",
                ExecutionDispositionLedgerV2::completion_path(root).display()
            ),
        );
    }
    if let Err(why) = preflight_files(&paths, bounds.max_file_bytes) {
        return StageProbe::refused(expected, why);
    }
    let mut ledger = match ExecutionDispositionLedgerV2::open_read(root) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(expected, why),
    };
    let mut facts = Vec::with_capacity(POPULATION_COUNT);
    let mut identities = Vec::with_capacity(POPULATION_COUNT * 4);
    for population_id in request.population_ids {
        let completion = match ledger.completion(population_id) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return StageProbe::unmeasured(
                    identities,
                    format!(
                        "Execution V2 completion for {} is absent",
                        hex(&population_id)
                    ),
                );
            }
            Err(why) => return StageProbe::refused(identities, why),
        };
        identities.push(labeled_id("population", population_id));
        identities.push(labeled_id("completion", completion.completion_id()));
        identities.push(labeled_id("long-parameter", completion.long_parameter_id()));
        identities.push(labeled_id(
            "short-parameter",
            completion.short_parameter_id(),
        ));
        facts.push(ExecutionFact {
            population_id,
            population_digest: completion.population_v4_digest(),
            admission_digest: completion.admission_completion_digest(),
            completion_id: completion.completion_id(),
            parameter_ids: completion.parameter_ids(),
            row_count: completion.row_count(),
            authorized: completion.authorized_capability_count(),
            policy_refused: completion.policy_refused_count(),
            matrix: completion.admission_execution_matrix(),
        });
    }
    execution_probe(facts, identities)
}

fn execution_probe(
    facts: Vec<ExecutionFact>,
    identities: Vec<String>,
) -> StageProbe<Vec<ExecutionFact>> {
    let rows = sum_or_refuse(&facts, &identities, |fact| fact.row_count, "execution rows");
    let authorized = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.authorized,
        "authorized rows",
    );
    let refused = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.policy_refused,
        "policy-refused rows",
    );
    let (Ok(rows), Ok(authorized), Ok(refused)) = (rows, authorized, refused) else {
        return StageProbe::refused(identities, "execution aggregate count overflowed");
    };
    StageProbe::ready(
        facts,
        identities,
        vec![
            Step3NamedCountV1::new("receipts", POPULATION_COUNT as u64),
            Step3NamedCountV1::new("dispositions", rows),
            Step3NamedCountV1::new("authorized", authorized),
            Step3NamedCountV1::new("policy-refused", refused),
        ],
        "all 16 Execution V2 completions and side-specific parameters reopened unchanged",
    )
}

fn read_stored(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<Vec<StoredFact>> {
    let expected = labeled_ids("population", &request.population_ids);
    let primary = StoredDataCompletenessLedgerV1::path(root);
    if !primary.exists() {
        return StageProbe::unmeasured(expected, format!("{} is absent", primary.display()));
    }
    if let Err(why) = preflight_files(std::slice::from_ref(&primary), bounds.max_file_bytes) {
        return StageProbe::refused(expected, why);
    }
    let mut ledger =
        match StoredDataCompletenessLedgerV1::open_read(root, bounds.max_stored_receipts) {
            Ok(value) => value,
            Err(why) => return StageProbe::refused(expected, why),
        };
    let mut facts = Vec::with_capacity(POPULATION_COUNT);
    let mut identities = Vec::with_capacity(POPULATION_COUNT * 3);
    for population_id in request.population_ids {
        let authority = match ledger.authority(population_id) {
            Ok(value) => value,
            Err(why) if why.contains("is not committed") => {
                return StageProbe::unmeasured(identities, why);
            }
            Err(why) => return StageProbe::refused(identities, why),
        };
        let receipt = authority.receipt();
        let receipt_digest = match receipt.content_digest() {
            Ok(value) => value,
            Err(why) => return StageProbe::refused(identities, why),
        };
        identities.push(labeled_id("population", population_id));
        identities.push(labeled_id("receipt", receipt_digest));
        identities.push(labeled_id("data", receipt.data_digest()));
        facts.push(StoredFact {
            population_id,
            population_digest: receipt.population_v4_digest(),
            receipt_digest,
            data_digest: receipt.data_digest(),
            signal_records: receipt.signal_records(),
            minute_context_records: receipt.minute_context_records(),
            execution_records: receipt.execution_records(),
            daily_records: receipt.daily_records(),
        });
    }
    stored_probe(facts, identities, ledger.receipts())
}

fn stored_probe(
    facts: Vec<StoredFact>,
    identities: Vec<String>,
    ledger_receipts: usize,
) -> StageProbe<Vec<StoredFact>> {
    let signal = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.signal_records,
        "signal records",
    );
    let context = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.minute_context_records,
        "minute-context records",
    );
    let execution = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.execution_records,
        "execution records",
    );
    let daily = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.daily_records,
        "daily-reference records",
    );
    let (Ok(signal), Ok(context), Ok(execution), Ok(daily)) = (signal, context, execution, daily)
    else {
        return StageProbe::refused(identities, "stored-data aggregate count overflowed");
    };
    StageProbe::ready(
        facts,
        identities,
        vec![
            Step3NamedCountV1::new("requested-receipts", POPULATION_COUNT as u64),
            Step3NamedCountV1::new("ledger-receipts", ledger_receipts as u64),
            Step3NamedCountV1::new("signal-records", signal),
            Step3NamedCountV1::new("minute-context-records", context),
            Step3NamedCountV1::new("execution-records", execution),
            Step3NamedCountV1::new("daily-reference-records", daily),
        ],
        "all 16 typed stored-data receipts bind exact signal, one-minute and daily-reference bytes",
    )
}

fn read_selections(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<Vec<SelectionFact>> {
    let expected = labeled_ids("selection", &request.selection_ids);
    let primary = SelectionLedgerV4::path(root);
    if !primary.exists() {
        return StageProbe::unmeasured(expected, format!("{} is absent", primary.display()));
    }
    if let Err(why) = preflight_files(std::slice::from_ref(&primary), bounds.max_file_bytes) {
        return StageProbe::refused(expected, why);
    }
    let ledger = match SelectionLedgerV4::open_read(root, bounds.max_selection_receipts) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(expected, why),
    };
    let mut facts = Vec::with_capacity(SELECTION_COUNT);
    let mut identities = Vec::with_capacity(SELECTION_COUNT);
    for (index, selection_id) in request.selection_ids.iter().copied().enumerate() {
        let Some(receipt) = ledger.receipt(&selection_id) else {
            return StageProbe::unmeasured(
                identities,
                format!("Selection V4 receipt {} is absent", hex(&selection_id)),
            );
        };
        let Some(expected_rung) = GLOBAL_REPLAY_V2_RUNGS_SECONDS.get(index).copied() else {
            return StageProbe::refused(
                identities,
                format!("Selection V4 canonical rung slot {index} is absent"),
            );
        };
        if receipt.rung_seconds() != expected_rung {
            return StageProbe::refused(
                identities,
                format!(
                    "Selection V4 slot {index} expected {expected_rung}s, found {}s",
                    receipt.rung_seconds()
                ),
            );
        }
        identities.push(labeled_id("selection", selection_id));
        facts.push(selection_fact(receipt));
    }
    selection_probe(facts, identities)
}

fn selection_fact(receipt: &SelectionReceiptV4) -> SelectionFact {
    SelectionFact {
        selection_id: receipt.selection_id(),
        rung_seconds: receipt.rung_seconds(),
        references: *receipt.populations(),
        top_ten: receipt.top_ten().len() as u64,
        top_twenty_five: receipt.top_twenty_five().len() as u64,
    }
}

fn selection_probe(
    facts: Vec<SelectionFact>,
    identities: Vec<String>,
) -> StageProbe<Vec<SelectionFact>> {
    let top_ten = sum_or_refuse(&facts, &identities, |fact| fact.top_ten, "Top-10 rows");
    let top_twenty_five = sum_or_refuse(
        &facts,
        &identities,
        |fact| fact.top_twenty_five,
        "Top-25 rows",
    );
    let (Ok(top_ten), Ok(top_twenty_five)) = (top_ten, top_twenty_five) else {
        return StageProbe::refused(identities, "Selection V4 aggregate count overflowed");
    };
    StageProbe::ready(
        facts,
        identities,
        vec![
            Step3NamedCountV1::new("receipts", SELECTION_COUNT as u64),
            Step3NamedCountV1::new("top10", top_ten),
            Step3NamedCountV1::new("top25", top_twenty_five),
            Step3NamedCountV1::new("8x25-capacity", GLOBAL_STREAM_COUNT),
        ],
        "all eight Selection V4 receipts reopened in canonical rung order",
    )
}

fn read_global_replay(
    root: &Path,
    request: &Step3AuthorityRequestV1,
    bounds: Step3ReadBoundsV1,
) -> StageProbe<crate::global_replay_v2::PreparedGlobalReplayV2> {
    let expected = vec![labeled_id("completion", request.replay_completion_id)];
    let primary = GlobalReplayLedgerV2::completion_path(root);
    if !primary.exists() {
        return StageProbe::unmeasured(expected, format!("{} is absent", primary.display()));
    }
    if let Err(why) = preflight_files(&global_replay_paths(root), bounds.max_file_bytes) {
        return StageProbe::refused(expected, why);
    }
    let ledger = match GlobalReplayLedgerV2::open_read(root, bounds.max_replay_completions) {
        Ok(value) => value,
        Err(why) => return StageProbe::refused(expected, why),
    };
    let Some(completion) = ledger.completion(&request.replay_completion_id).copied() else {
        return StageProbe::unmeasured(
            expected,
            format!(
                "Global Replay V2 completion {} is absent",
                hex(&request.replay_completion_id)
            ),
        );
    };
    let Some(prepared) = ledger.replay(&completion.replay_id()).cloned() else {
        return StageProbe::refused(
            expected,
            "Global Replay V2 completion has no reconstructed replay",
        );
    };
    let counters = completion.counters();
    let identities = vec![
        labeled_id("completion", completion.completion_id()),
        labeled_id("replay", completion.replay_id()),
        labeled_id("manifest", completion.manifest_id()),
    ];
    StageProbe::ready(
        prepared,
        identities,
        replay_counts(
            completion.stream_count(),
            completion.decision_count(),
            counters,
        ),
        "the Global Replay V2 execution-only receipt and all reconstructed rows reopened",
    )
}

fn replay_counts(streams: u64, decisions: u64, counters: Counters) -> Vec<Step3NamedCountV1> {
    vec![
        Step3NamedCountV1::new("streams", streams),
        Step3NamedCountV1::new("decisions", decisions),
        Step3NamedCountV1::new("offered", counters.offered),
        Step3NamedCountV1::new("admitted", counters.admitted),
        Step3NamedCountV1::new("blocked-occupied", counters.blocked_occupied),
        Step3NamedCountV1::new("blocked-simultaneous", counters.blocked_simultaneous),
        Step3NamedCountV1::new("unreachable", counters.unreachable),
        Step3NamedCountV1::new("refused", counters.refused),
    ]
}

fn reconcile_admission(
    population: &StageProbe<Vec<PopulationFact>>,
    admission: &mut StageProbe<Vec<AdmissionFact>>,
) {
    if !admission.is_ready() {
        return;
    }
    let Some(populations) = population.evidence.as_deref() else {
        admission.block("admission bytes are valid, but Population V4 is not ready");
        return;
    };
    let Some(admissions) = admission.evidence.as_deref() else {
        admission.refuse("admission ready state carries no evidence");
        return;
    };
    if let Err(why) = validate_admission_bindings(populations, admissions) {
        admission.refuse(why);
    } else {
        "all admission IDs, Population V4 digests and exhaustive counts reconcile"
            .clone_into(&mut admission.detail);
    }
}

fn validate_admission_bindings(
    populations: &[PopulationFact],
    admissions: &[AdmissionFact],
) -> Result<(), String> {
    require_parallel_len(populations.len(), admissions.len(), "admission")?;
    for (population, admission) in populations.iter().zip(admissions) {
        if admission.population_id != population.population_id
            || admission.population_digest != population.completion_digest
        {
            return Err(format!(
                "admission authority for {} binds a foreign Population V4 receipt",
                hex(&population.population_id)
            ));
        }
        if admission.decision_count != population.row_count {
            return Err(format!(
                "admission count {} differs from Population V4 row count {} for {}",
                admission.decision_count,
                population.row_count,
                hex(&population.population_id)
            ));
        }
        let accounted = checked_sum(
            [
                admission.admitted,
                admission.rejected,
                admission.unmeasured,
                admission.refused,
            ],
            "admission terminal counts",
        )?;
        if accounted != admission.decision_count {
            return Err("admission terminal counts do not cover every decision".to_owned());
        }
    }
    Ok(())
}

fn reconcile_execution(
    population: &StageProbe<Vec<PopulationFact>>,
    admission: &StageProbe<Vec<AdmissionFact>>,
    execution: &mut StageProbe<Vec<ExecutionFact>>,
) {
    if !execution.is_ready() {
        return;
    }
    let (Some(populations), Some(admissions)) = (
        population.evidence.as_deref(),
        admission.evidence.as_deref(),
    ) else {
        execution.block("Execution V2 bytes are valid, but population/admission is not ready");
        return;
    };
    let Some(executions) = execution.evidence.as_deref() else {
        execution.refuse("Execution V2 ready state carries no evidence");
        return;
    };
    if let Err(why) = validate_execution_bindings(populations, admissions, executions) {
        execution.refuse(why);
    } else {
        "all Execution V2 population/admission IDs, counts, matrices and parameters reconcile"
            .clone_into(&mut execution.detail);
    }
}

fn validate_execution_bindings(
    populations: &[PopulationFact],
    admissions: &[AdmissionFact],
    executions: &[ExecutionFact],
) -> Result<(), String> {
    require_parallel_len(populations.len(), executions.len(), "execution")?;
    require_parallel_len(admissions.len(), executions.len(), "execution admission")?;
    for ((population, admission), execution) in populations.iter().zip(admissions).zip(executions) {
        if execution.population_id != population.population_id
            || execution.population_digest != population.completion_digest
            || execution.admission_digest != admission.completion_digest
        {
            return Err(format!(
                "Execution V2 authority for {} binds a foreign prerequisite",
                hex(&population.population_id)
            ));
        }
        if execution.row_count != population.row_count {
            return Err("Execution V2 row count differs from Population V4".to_owned());
        }
        if execution.parameter_ids[0] == execution.parameter_ids[1] {
            return Err("Execution V2 copied one parameter across long and short".to_owned());
        }
        let disposition_total = execution
            .authorized
            .checked_add(execution.policy_refused)
            .ok_or_else(|| "Execution V2 terminal count overflowed".to_owned())?;
        let matrix_total = checked_sum(execution.matrix.counts(), "Execution V2 matrix")?;
        if disposition_total != execution.row_count || matrix_total != execution.row_count {
            return Err("Execution V2 counts do not cover every population row".to_owned());
        }
    }
    Ok(())
}

fn reconcile_stored(
    population: &StageProbe<Vec<PopulationFact>>,
    stored: &mut StageProbe<Vec<StoredFact>>,
) {
    if !stored.is_ready() {
        return;
    }
    let Some(populations) = population.evidence.as_deref() else {
        stored.block("stored-data bytes are valid, but Population V4 is not ready");
        return;
    };
    let Some(stored_facts) = stored.evidence.as_deref() else {
        stored.refuse("stored-data ready state carries no evidence");
        return;
    };
    if let Err(why) = validate_stored_bindings(populations, stored_facts) {
        stored.refuse(why);
    } else {
        "all stored signal, full one-minute context, execution and daily-reference receipts bind the same Population V4 IDs"
            .clone_into(&mut stored.detail);
    }
}

fn validate_stored_bindings(
    populations: &[PopulationFact],
    stored: &[StoredFact],
) -> Result<(), String> {
    require_parallel_len(populations.len(), stored.len(), "stored-data")?;
    for (population, stored) in populations.iter().zip(stored) {
        if stored.population_id != population.population_id
            || stored.population_digest != population.completion_digest
        {
            return Err(format!(
                "stored-data receipt for {} binds a foreign Population V4 receipt",
                hex(&population.population_id)
            ));
        }
        for (name, digest) in [
            ("stored receipt", stored.receipt_digest),
            ("stored data", stored.data_digest),
        ] {
            require_id(name, digest)?;
        }
    }
    Ok(())
}

fn reconcile_selections(
    population: &StageProbe<Vec<PopulationFact>>,
    admission: &StageProbe<Vec<AdmissionFact>>,
    execution: &StageProbe<Vec<ExecutionFact>>,
    selection: &mut StageProbe<Vec<SelectionFact>>,
) {
    if !selection.is_ready() {
        return;
    }
    let (Some(populations), Some(admissions), Some(executions)) = (
        population.evidence.as_deref(),
        admission.evidence.as_deref(),
        execution.evidence.as_deref(),
    ) else {
        selection.block(
            "Selection V4 bytes are valid, but a triple-authority prerequisite is not ready",
        );
        return;
    };
    let Some(selections) = selection.evidence.as_deref() else {
        selection.refuse("Selection V4 ready state carries no evidence");
        return;
    };
    if let Err(why) = validate_selection_bindings(populations, admissions, executions, selections) {
        selection.refuse(why);
        return;
    }
    finalize_structural_selection(selection, validate_selection_topology(selections));
}

fn finalize_structural_selection<T>(selection: &mut StageProbe<T>, topology: Result<(), String>) {
    match topology {
        Ok(()) => selection.block(
            "eight structurally valid Selection V4 receipts bind the same 16 triple-authority references and expose exact Top-10/Top-25 prefixes, but READY requires SelectionReceiptV4::verify_against_authorities with the exact RankingPolicyV1; no policy or live replay capability was supplied",
        ),
        Err(why) => selection.block(why),
    }
}

fn validate_selection_bindings(
    populations: &[PopulationFact],
    admissions: &[AdmissionFact],
    executions: &[ExecutionFact],
    selections: &[SelectionFact],
) -> Result<(), String> {
    require_parallel_len(selections.len(), SELECTION_COUNT, "Selection V4")?;
    require_parallel_len(
        populations.len(),
        POPULATION_COUNT,
        "Selection V4 populations",
    )?;
    for (rung_index, selection) in selections.iter().enumerate() {
        let expected_rung = GLOBAL_REPLAY_V2_RUNGS_SECONDS
            .get(rung_index)
            .copied()
            .ok_or_else(|| "Selection V4 canonical rung slot is absent".to_owned())?;
        if selection.rung_seconds != expected_rung {
            return Err("Selection V4 rung order is noncanonical".to_owned());
        }
        for family_index in 0..POPULATIONS_PER_SELECTION {
            let slot = rung_index * POPULATIONS_PER_SELECTION + family_index;
            validate_selection_reference(
                selection
                    .references
                    .get(family_index)
                    .ok_or_else(|| "Selection V4 family reference is absent".to_owned())?,
                populations
                    .get(slot)
                    .ok_or_else(|| "Selection V4 population slot is absent".to_owned())?,
                admissions
                    .get(slot)
                    .ok_or_else(|| "Selection V4 admission slot is absent".to_owned())?,
                executions
                    .get(slot)
                    .ok_or_else(|| "Selection V4 execution slot is absent".to_owned())?,
            )?;
        }
    }
    Ok(())
}

fn validate_selection_reference(
    reference: &PopulationReferenceV4,
    population: &PopulationFact,
    admission: &AdmissionFact,
    execution: &ExecutionFact,
) -> Result<(), String> {
    if reference.population_id() != population.population_id
        || reference.family() != population.family
        || reference.row_count() != population.row_count
        || reference.ordered_row_digest() != population.ordered_row_digest
        || reference.population_v4_completion_digest() != population.completion_digest
        || reference.admission_completion_digest() != admission.completion_digest
        || reference.execution_v2_completion_digest() != execution.completion_id
    {
        return Err(format!(
            "Selection V4 reference names a foreign authority instead of {}",
            hex(&population.population_id)
        ));
    }
    Ok(())
}

fn validate_selection_topology(selections: &[SelectionFact]) -> Result<(), String> {
    validate_selection_topology_counts(
        selections
            .iter()
            .map(|selection| (selection.top_ten, selection.top_twenty_five)),
    )
}

fn validate_selection_topology_counts(
    counts: impl IntoIterator<Item = (u64, u64)>,
) -> Result<(), String> {
    let mut observed = 0_usize;
    let mut top_ten = 0_u64;
    let mut top_twenty_five = 0_u64;
    for (ten, twenty_five) in counts {
        observed = observed
            .checked_add(1)
            .ok_or_else(|| "Selection V4 topology count overflowed usize".to_owned())?;
        top_ten = top_ten
            .checked_add(ten)
            .ok_or_else(|| "Top-10 overflowed u64".to_owned())?;
        top_twenty_five = top_twenty_five
            .checked_add(twenty_five)
            .ok_or_else(|| "Top-25 overflowed u64".to_owned())?;
    }
    require_parallel_len(observed, SELECTION_COUNT, "Selection V4 topology")?;
    let expected_top_ten = (SELECTION_COUNT as u64) * TOP_TEN_PER_SELECTION;
    let expected_top_twenty_five = (SELECTION_COUNT as u64) * TOP_TWENTY_FIVE_PER_SELECTION;
    if top_ten != expected_top_ten || top_twenty_five != expected_top_twenty_five {
        return Err(format!(
            "full replay requires Top-10={expected_top_ten} and Top-25={expected_top_twenty_five}; observed {top_ten}/{top_twenty_five}"
        ));
    }
    Ok(())
}

fn reconcile_global_replay(
    request: &Step3AuthorityRequestV1,
    population: &StageProbe<Vec<PopulationFact>>,
    admission: &StageProbe<Vec<AdmissionFact>>,
    execution: &StageProbe<Vec<ExecutionFact>>,
    selection: &StageProbe<Vec<SelectionFact>>,
    replay: &mut StageProbe<crate::global_replay_v2::PreparedGlobalReplayV2>,
) {
    if !replay.is_ready() {
        return;
    }
    let (Some(populations), Some(admissions), Some(executions), Some(selections)) = (
        population.evidence.as_deref(),
        admission.evidence.as_deref(),
        execution.evidence.as_deref(),
        selection.evidence.as_deref(),
    ) else {
        replay.block(
            "Global Replay V2 bytes are valid, but Selection V4 or its prerequisites are not ready",
        );
        return;
    };
    let Some(prepared) = replay.evidence.as_ref() else {
        replay.refuse("Global Replay V2 ready state carries no evidence");
        return;
    };
    if let Err(why) = validate_global_bindings(
        request,
        populations,
        admissions,
        executions,
        selections,
        prepared,
    ) {
        replay.refuse(why);
    } else {
        "exact 8×25 streams and every scheduler decision reconcile to the same Selection V4/Execution V2 manifest; scope remains execution-only"
            .clone_into(&mut replay.detail);
    }
}

fn validate_global_bindings(
    request: &Step3AuthorityRequestV1,
    populations: &[PopulationFact],
    admissions: &[AdmissionFact],
    executions: &[ExecutionFact],
    selections: &[SelectionFact],
    prepared: &crate::global_replay_v2::PreparedGlobalReplayV2,
) -> Result<(), String> {
    let manifest = prepared.manifest();
    if manifest.selection_ids() != &request.selection_ids {
        return Err("Global Replay V2 manifest names foreign Selection V4 IDs".to_owned());
    }
    for (slot, ((population, admission), execution)) in manifest
        .authorities()
        .iter()
        .zip(populations.iter().zip(admissions).zip(executions))
    {
        if slot.population_id() != population.population_id
            || slot.population_v4_completion_digest() != population.completion_digest
            || slot.admission_completion_digest() != admission.completion_digest
            || slot.execution_v2_completion_id() != execution.completion_id
            || slot.execution_parameter_ids() != execution.parameter_ids
            || slot.row_count() != population.row_count
        {
            return Err("Global Replay V2 manifest carries a foreign authority slot".to_owned());
        }
    }
    if prepared.streams().len() as u64 != GLOBAL_STREAM_COUNT {
        return Err(format!(
            "Global Replay V2 has {} streams, not {GLOBAL_STREAM_COUNT}",
            prepared.streams().len()
        ));
    }
    let decisions = prepared.decisions().len() as u64;
    let counters = prepared.counters();
    if counters.offered != decisions || !counters.reconciles() {
        return Err("Global Replay V2 scheduler counters do not cover every decision".to_owned());
    }
    validate_selection_topology(selections)
}

fn expected_family(index: usize) -> InstrumentFamilyV1 {
    if index.is_multiple_of(2) {
        InstrumentFamilyV1::Nifty
    } else {
        InstrumentFamilyV1::BankNifty
    }
}

fn execution_paths(root: &Path) -> [PathBuf; 4] {
    [
        ExecutionDispositionLedgerV2::parameter_path(root),
        ExecutionDispositionLedgerV2::percentile_path(root),
        ExecutionDispositionLedgerV2::row_path(root),
        ExecutionDispositionLedgerV2::completion_path(root),
    ]
}

fn global_replay_paths(root: &Path) -> [PathBuf; 5] {
    [
        GlobalReplayManifestV2::path(root),
        GlobalReplayLedgerV2::stream_path(root),
        GlobalReplayLedgerV2::candidate_path(root),
        GlobalReplayLedgerV2::decision_path(root),
        GlobalReplayLedgerV2::completion_path(root),
    ]
}

fn preflight_files(paths: &[PathBuf], max_file_bytes: u64) -> Result<(), String> {
    for path in paths {
        let metadata = fs::metadata(path)
            .map_err(|why| format!("{} metadata could not be read: {why}", path.display()))?;
        if !metadata.is_file() {
            return Err(format!("{} is not a regular file", path.display()));
        }
        if metadata.len() > max_file_bytes {
            return Err(format!(
                "{} is {} bytes, above Step-3 bound {max_file_bytes}",
                path.display(),
                metadata.len()
            ));
        }
    }
    Ok(())
}

fn validate_unique_ids<const N: usize>(name: &str, ids: &[[u8; 32]; N]) -> Result<(), String> {
    let mut seen = HashSet::with_capacity(N);
    for id in ids {
        require_id(name, *id)?;
        if !seen.insert(*id) {
            return Err(format!("Step-3 request repeats one {name} identity"));
        }
    }
    Ok(())
}

fn require_id(name: &str, id: [u8; 32]) -> Result<(), String> {
    if id == [0; 32] {
        Err(format!("{name} identity is zero"))
    } else {
        Ok(())
    }
}

fn require_parallel_len(actual: usize, expected: usize, name: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{name} has {actual} authority records, expected {expected}"
        ))
    }
}

fn checked_sum<const N: usize>(values: [u64; N], name: &str) -> Result<u64, String> {
    sum_u64(values, name)
}

fn sum_u64(values: impl IntoIterator<Item = u64>, name: &str) -> Result<u64, String> {
    values.into_iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(value)
            .ok_or_else(|| format!("{name} overflowed u64"))
    })
}

fn sum_or_refuse<T>(
    facts: &[T],
    _identities: &[String],
    value: impl Fn(&T) -> u64,
    name: &str,
) -> Result<u64, String> {
    sum_u64(facts.iter().map(value), name)
}

fn labeled_ids<const N: usize>(label: &str, ids: &[[u8; 32]; N]) -> Vec<String> {
    ids.iter().map(|id| labeled_id(label, *id)).collect()
}

fn labeled_id(label: &str, id: [u8; 32]) -> String {
    format!("{label}={}", hex(&id))
}

fn hex(bytes: &[u8; 32]) -> String {
    let mut value = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    reason = "fixed-size test identities stay below u8 and fixture failures must identify their exact setup boundary"
)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write as _;
    use std::time::{SystemTime, UNIX_EPOCH};

    type RefusalCase<T> = (&'static str, fn(&mut T));

    fn id(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn request() -> Step3AuthorityRequestV1 {
        let populations = std::array::from_fn(|index| id((index + 1) as u8));
        let selections = std::array::from_fn(|index| id((index + 33) as u8));
        Step3AuthorityRequestV1::new(populations, selections, id(64)).expect("request")
    }

    fn bounds() -> Step3ReadBoundsV1 {
        Step3ReadBoundsV1::new(1_000_000, 8, 16, 1).expect("bounds")
    }

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "brutex-step3-comparison-{}-{name}-{nonce}",
            std::process::id()
        ))
    }

    fn population_facts(request: &Step3AuthorityRequestV1) -> Vec<PopulationFact> {
        request
            .population_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(index, population_id)| {
                let rung_seconds = GLOBAL_REPLAY_V2_RUNGS_SECONDS
                    .get(index / 2)
                    .copied()
                    .expect("population rung");
                PopulationFact {
                    population_id,
                    completion_digest: id((index + 80) as u8),
                    ordered_row_digest: id((index + 100) as u8),
                    row_count: 2,
                    rung_seconds,
                    family: expected_family(index),
                }
            })
            .collect()
    }

    fn admission_facts(populations: &[PopulationFact]) -> Vec<AdmissionFact> {
        populations
            .iter()
            .enumerate()
            .map(|(index, population)| AdmissionFact {
                population_id: population.population_id,
                population_digest: population.completion_digest,
                completion_digest: id((index + 120) as u8),
                decision_count: population.row_count,
                admitted: 1,
                rejected: 1,
                unmeasured: 0,
                refused: 0,
            })
            .collect()
    }

    fn stored_facts(populations: &[PopulationFact]) -> Vec<StoredFact> {
        populations
            .iter()
            .map(|population| StoredFact {
                population_id: population.population_id,
                population_digest: population.completion_digest,
                receipt_digest: id(200),
                data_digest: id(201),
                signal_records: 2,
                minute_context_records: 3,
                execution_records: 4,
                daily_records: 5,
            })
            .collect()
    }

    #[test]
    fn each_corrupt_stage_is_refused_without_hiding_the_other_five_states() {
        let stages = [
            Step3StageV1::PopulationV4,
            Step3StageV1::Admission,
            Step3StageV1::ExecutionV2,
            Step3StageV1::StoredData,
            Step3StageV1::SelectionV4,
            Step3StageV1::GlobalReplayV2,
        ];
        for stage in stages {
            let root = temp_root(stage.label());
            let paths = match stage {
                Step3StageV1::PopulationV4 => vec![PopulationLedger::receipt_v4_path(&root)],
                Step3StageV1::Admission => vec![AdmissionAuthorityLedger::completion_path(&root)],
                Step3StageV1::ExecutionV2 => execution_paths(&root).to_vec(),
                Step3StageV1::StoredData => vec![StoredDataCompletenessLedgerV1::path(&root)],
                Step3StageV1::SelectionV4 => vec![SelectionLedgerV4::path(&root)],
                Step3StageV1::GlobalReplayV2 => global_replay_paths(&root).to_vec(),
            };
            for path in &paths {
                fs::create_dir_all(path.parent().expect("stage parent")).expect("parent");
                fs::write(path, b"corrupt authority").expect("corrupt stage fixture");
            }
            let report = compare_step3_on_disk_v1(&root, &request(), bounds()).expect("comparison");
            assert_eq!(report.rows().len(), 6);
            assert!(!report.is_ready());
            for row in report.rows() {
                let expected = if row.stage() == stage {
                    Step3StatusV1::Refused
                } else {
                    Step3StatusV1::Unmeasured
                };
                assert_eq!(
                    row.status(),
                    expected,
                    "corrupted {}: {}",
                    stage.label(),
                    row.detail()
                );
                assert!(!row.identities().is_empty());
                assert!(row.counts().is_empty());
            }
            fs::remove_dir_all(root).expect("remove isolated stage fixture");
        }
    }

    #[test]
    fn file_preflight_refuses_missing_nonfiles_and_oversize_without_reading_them() {
        let root = temp_root("file-bounds");
        fs::create_dir_all(&root).expect("directory");
        let path = root.join("authority.bin");
        assert!(
            preflight_files(std::slice::from_ref(&path), 3)
                .expect_err("missing file")
                .contains("metadata could not be read")
        );
        assert!(
            preflight_files(std::slice::from_ref(&root), 3)
                .expect_err("directory is not a file")
                .contains("not a regular file")
        );
        fs::write(&path, [1, 2, 3]).expect("bounded bytes");
        preflight_files(std::slice::from_ref(&path), 3).expect("exact bound");
        assert!(
            preflight_files(std::slice::from_ref(&path), 2)
                .expect_err("one byte beyond bound")
                .contains("above Step-3 bound 2")
        );
        assert_eq!(fs::read(&path).expect("retained bytes"), [1, 2, 3]);
        fs::remove_dir_all(root).expect("remove preflight fixture");
    }

    #[test]
    fn admission_reconciliation_refuses_foreign_incomplete_and_overflowed_evidence() {
        let populations = population_facts(&request());
        let population =
            StageProbe::ready(populations.clone(), Vec::new(), Vec::new(), "generated");
        let facts = admission_facts(&populations);
        let mut good = admission_probe(facts.clone(), vec!["generated admission".to_owned()]);
        reconcile_admission(&population, &mut good);
        assert_eq!(good.status, Step3StatusV1::Ready);
        assert_eq!(
            good.counts
                .iter()
                .map(|count| (count.name(), count.value()))
                .collect::<Vec<_>>(),
            [
                ("receipts", 16),
                ("decisions", 32),
                ("admitted", 16),
                ("rejected", 16),
                ("unmeasured", 0),
                ("refused", 0)
            ]
        );

        let cases: [RefusalCase<AdmissionFact>; 5] = [
            ("foreign Population V4", |fact| fact.population_id = id(250)),
            ("foreign Population V4", |fact| {
                fact.population_digest = id(250);
            }),
            ("differs from Population V4 row count", |fact| {
                fact.decision_count = 3;
            }),
            ("do not cover every decision", |fact| fact.admitted = 0),
            ("overflowed u64", |fact| fact.admitted = u64::MAX),
        ];
        for (reason, change) in cases {
            let mut changed = facts.clone();
            change(changed.last_mut().expect("last admission"));
            let mut probe = StageProbe::ready(changed, Vec::new(), Vec::new(), "generated");
            reconcile_admission(&population, &mut probe);
            assert_eq!(probe.status, Step3StatusV1::Refused);
            assert!(probe.evidence.is_none());
            assert!(probe.detail.contains(reason), "{}", probe.detail);
        }
        let mut short = facts.clone();
        short.pop();
        assert!(validate_admission_bindings(&populations, &short).is_err());
        let absent: StageProbe<Vec<PopulationFact>> = StageProbe::unmeasured(Vec::new(), "absent");
        let mut blocked = admission_probe(facts, Vec::new());
        reconcile_admission(&absent, &mut blocked);
        assert_eq!(blocked.status, Step3StatusV1::Blocked);
        assert!(blocked.evidence.is_none());
        let mut impossible: StageProbe<Vec<AdmissionFact>> =
            StageProbe::ready(Vec::new(), Vec::new(), Vec::new(), "generated");
        impossible.evidence = None;
        reconcile_admission(&population, &mut impossible);
        assert_eq!(impossible.status, Step3StatusV1::Refused);
        assert!(impossible.detail.contains("no evidence"));
    }

    #[test]
    fn aggregate_admission_and_stored_counts_never_wrap_into_ready_rows() {
        let populations = population_facts(&request());
        let admission_fields: [fn(&mut AdmissionFact) -> &mut u64; 5] = [
            |fact| &mut fact.decision_count,
            |fact| &mut fact.admitted,
            |fact| &mut fact.rejected,
            |fact| &mut fact.unmeasured,
            |fact| &mut fact.refused,
        ];
        for field in admission_fields {
            let mut facts = admission_facts(&populations);
            *field(facts.first_mut().expect("first admission")) = u64::MAX;
            *field(facts.last_mut().expect("last admission")) = 1;
            let probe = admission_probe(facts, vec!["generated overflow".to_owned()]);
            assert_eq!(probe.status, Step3StatusV1::Refused);
            assert!(probe.detail.contains("overflowed"));
            assert!(probe.counts.is_empty());
        }
        let stored_fields: [fn(&mut StoredFact) -> &mut u64; 4] = [
            |fact| &mut fact.signal_records,
            |fact| &mut fact.minute_context_records,
            |fact| &mut fact.execution_records,
            |fact| &mut fact.daily_records,
        ];
        for field in stored_fields {
            let mut facts = stored_facts(&populations);
            *field(facts.first_mut().expect("first stored receipt")) = u64::MAX;
            let probe = stored_probe(facts, Vec::new(), 16);
            assert_eq!(probe.status, Step3StatusV1::Refused);
            assert!(probe.detail.contains("overflowed"));
            assert!(probe.counts.is_empty());
        }
    }

    #[test]
    fn stored_reconciliation_binds_both_identities_and_retains_exact_counts() {
        let populations = population_facts(&request());
        let facts = stored_facts(&populations);
        let population = StageProbe::ready(populations, Vec::new(), Vec::new(), "generated");
        let mut good = stored_probe(facts.clone(), vec!["generated stored".to_owned()], 17);
        reconcile_stored(&population, &mut good);
        assert_eq!(good.status, Step3StatusV1::Ready);
        assert_eq!(
            good.counts
                .iter()
                .map(|count| (count.name(), count.value()))
                .collect::<Vec<_>>(),
            [
                ("requested-receipts", 16),
                ("ledger-receipts", 17),
                ("signal-records", 32),
                ("minute-context-records", 48),
                ("execution-records", 64),
                ("daily-reference-records", 80)
            ]
        );
        let cases: [RefusalCase<StoredFact>; 4] = [
            ("foreign Population V4", |fact| fact.population_id = id(250)),
            ("foreign Population V4", |fact| {
                fact.population_digest = id(250);
            }),
            ("stored receipt identity is zero", |fact| {
                fact.receipt_digest = [0; 32];
            }),
            ("stored data identity is zero", |fact| {
                fact.data_digest = [0; 32];
            }),
        ];
        for (reason, change) in cases {
            let mut changed = facts.clone();
            change(changed.last_mut().expect("last stored receipt"));
            let mut probe = stored_probe(changed, Vec::new(), 16);
            reconcile_stored(&population, &mut probe);
            assert_eq!(probe.status, Step3StatusV1::Refused);
            assert!(probe.evidence.is_none());
            assert!(probe.detail.contains(reason), "{}", probe.detail);
        }
        let absent: StageProbe<Vec<PopulationFact>> = StageProbe::unmeasured(Vec::new(), "absent");
        let mut blocked = stored_probe(facts, Vec::new(), 16);
        reconcile_stored(&absent, &mut blocked);
        assert_eq!(blocked.status, Step3StatusV1::Blocked);
        assert!(blocked.evidence.is_none());
    }

    #[test]
    fn absent_current_format_stages_are_unmeasured_without_fallback() {
        let root = temp_root("absent");
        let report = compare_step3_on_disk_v1(&root, &request(), bounds()).expect("comparison");
        assert_eq!(report.rows().len(), 6);
        assert!(
            report
                .rows()
                .iter()
                .all(|row| row.status() == Step3StatusV1::Unmeasured)
        );
        assert!(!report.is_ready());
    }

    #[test]
    fn corrupt_existing_population_stage_is_refused() {
        let root = temp_root("corrupt");
        let path = PopulationLedger::receipt_v4_path(&root);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        let mut file = File::create(&path).expect("file");
        file.write_all(b"not-a-population-v4-ledger")
            .expect("corrupt bytes");
        let report = compare_step3_on_disk_v1(&root, &request(), bounds()).expect("comparison");
        let population = report.rows().first().expect("population row");
        assert_eq!(population.status(), Step3StatusV1::Refused);
        assert!(population.detail().contains("could not be opened"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn consistent_population_and_admission_facts_reconcile() {
        let request = request();
        let populations = population_facts(&request);
        let admissions = admission_facts(&populations);
        validate_admission_bindings(&populations, &admissions).expect("consistent authority");
    }

    #[test]
    fn foreign_admission_identity_is_refused() {
        let request = request();
        let populations = population_facts(&request);
        let mut admissions = admission_facts(&populations);
        admissions
            .get_mut(7)
            .expect("eighth admission")
            .population_digest = id(250);
        let why = validate_admission_bindings(&populations, &admissions).expect_err("foreign");
        assert!(why.contains("foreign Population V4"));
    }

    #[test]
    fn stale_or_corrupt_probe_is_never_ready() {
        let mut probe = StageProbe::ready((), Vec::new(), Vec::new(), "opened");
        probe.refuse("stale generation or corrupt seal");
        assert_eq!(probe.status, Step3StatusV1::Refused);
        assert!(probe.evidence.is_none());
    }

    #[test]
    fn exact_top10_top25_and_8x25_counts_are_required() {
        let complete = vec![(10, 25); SELECTION_COUNT];
        validate_selection_topology_counts(complete.iter().copied()).expect("8x25");
        let mut short = complete;
        *short.get_mut(7).expect("eighth selection") = (9, 24);
        let why =
            validate_selection_topology_counts(short.iter().copied()).expect_err("short surface");
        assert!(why.contains("Top-10=80"));
        assert!(why.contains("Top-25=200"));
    }

    #[test]
    fn structurally_consistent_selection_stays_blocked_without_full_authority_replay() {
        let mut selection = StageProbe::ready(
            (),
            vec![labeled_id("selection", id(91))],
            vec![
                Step3NamedCountV1::new("top10", 80),
                Step3NamedCountV1::new("top25", 200),
            ],
            "structural references and topology reconciled",
        );
        finalize_structural_selection(&mut selection, Ok(()));
        assert_eq!(selection.status, Step3StatusV1::Blocked);
        assert!(selection.evidence.is_none());
        assert!(
            selection
                .detail
                .contains("verify_against_authorities with the exact RankingPolicyV1")
        );
    }

    #[test]
    fn renderer_is_deterministic_and_keeps_all_four_states_distinct() {
        let statuses = [
            Step3StatusV1::Ready,
            Step3StatusV1::Blocked,
            Step3StatusV1::Unmeasured,
            Step3StatusV1::Refused,
        ];
        let rows = statuses
            .into_iter()
            .enumerate()
            .map(|(index, status)| Step3ComparisonRowV1 {
                stage: Step3StageV1::PopulationV4,
                status,
                identities: vec![labeled_id("id", id((index + 1) as u8))],
                counts: vec![Step3NamedCountV1::new("rows", index as u64)],
                detail: "exact | detail".to_owned(),
            })
            .collect();
        let report = Step3ComparisonV1 { rows };
        let first = report.render_markdown();
        assert_eq!(first, report.render_markdown());
        for state in ["READY", "BLOCKED", "UNMEASURED", "REFUSED"] {
            assert!(first.contains(state));
        }
        assert!(first.contains("exact \\| detail"));
        assert!(first.contains(&hex(&id(1))));
    }

    #[test]
    fn request_and_bounds_refuse_zero_duplicate_and_undersized_inputs() {
        let populations = std::array::from_fn(|index| id((index + 1) as u8));
        let mut selections = std::array::from_fn(|index| id((index + 33) as u8));
        let first = selections.first().copied().expect("first selection");
        *selections.get_mut(7).expect("eighth selection") = first;
        assert!(Step3AuthorityRequestV1::new(populations, selections, id(64)).is_err());
        assert!(Step3ReadBoundsV1::new(1, 7, 16, 1).is_err());
        assert!(Step3ReadBoundsV1::new(1, 8, 15, 1).is_err());
        assert!(Step3ReadBoundsV1::new(1, 8, 16, 0).is_err());
    }
}
