//! Receipt-last V2 execution disposition authority.
//!
//! Population V4 deliberately contains every evaluated exit-grid cell.  The
//! V1 execution-capability ledger, however, required one policy-authorized
//! [`ExecutionStrategyCapabilityV1`] for every row.  A measured cell that the
//! exit policy refuses is a legitimate member of the complete population, not
//! a structural error and not an authorized selection.  Requiring a selected
//! capability for that row made the two complete authorities impossible to
//! join outside all-authorized fixtures.
//!
//! This module persists exactly one [`ExecutionRowDispositionV2`] for every
//! Population V4 row.  The row is either `Authorized`, with one sparse opaque
//! execution capability, or `PolicyRefused`, with the exact versioned runner
//! refusal bits.  Structural failures remain `Err` and cannot be serialized as
//! a policy refusal.  A completion receipt is appended only after the complete
//! ordered row block is durable.  It binds Population V4, the recomputed
//! admission completion, the exact execution law, the sparse capability count,
//! and the complete four-by-two admission/execution count matrix.
//!
//! # Cost
//!
//! Preparation and reopen are O(rows) and retain O(rows) indexes.  One exact
//! completion or row probe is average O(1) after open; one page is O(requested)
//! with a fixed 256-row ceiling.  File open, hashing, append, sync, Population
//! V4/admission reconciliation and end-to-end execution are not O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use runner::admission::AdmissionStatusV1;
use runner::excursion::Side;
use runner::exit_grid_policy::{ExecutionDispositionV1, ExecutionRefusalBitsV1};
use runner::grid::{Chosen, Ttp};

use crate::admission_store::{
    AdmissionAuthorityLedger, AdmissionCompletionReceiptV1, AdmissionDecisionRecordV1,
    MAX_ADMISSION_PAGE_ROWS_V1,
};
use crate::execution_capability::{
    EXECUTION_PARAMETER_STRIDE, EXECUTION_PERCENTILE_STRIDE, ExecutionParametersV1,
    ExecutionStrategyCapabilityV1, ParameterScalarV1, PercentileRecordV1,
    exact_execution_law_digest_v1, parameter_digest_records, percentile_digest_records,
};
use crate::population::{CompletionReceiptV4, PopulationLedger, PopulationRowV1, TradeDirectionV1};
use crate::population_admission_writer::derive_strategy_digest_from_disposition_v1;

/// Operator-facing refusal from the V2 execution-disposition authority.
pub type ExecutionDispositionRefusalV2 = String;

const HEADER_BYTES: u64 = 24;
const HEADER_BYTES_USIZE: usize = 24;
const FORMAT_VERSION_V2: u32 = 2;
const SEAL_BYTES: usize = 32;
const PARAMETER_MAGIC: [u8; 8] = *b"BRUX2PR1";
const PERCENTILE_MAGIC: [u8; 8] = *b"BRUX2PC1";
const ROW_MAGIC: [u8; 8] = *b"BRUTXED2";
const COMPLETION_MAGIC: [u8; 8] = *b"BRUTXEF2";
// 13 digests + 2 u64 + 2 tags + 5 optional-coordinate u64 + 6 reserve.
const ROW_PAYLOAD_BYTES: usize = 13 * 32 + 2 * 8 + 2 + 5 * 8 + 6;
// 10 digests + 9 u64 + the 8-u64 matrix + 56 reserve.
const COMPLETION_PAYLOAD_BYTES: usize = 10 * 32 + 9 * 8 + 8 * 8 + 56;
const ROW_ID_DOMAIN: &[u8] = b"brutex.cli.execution-row-disposition.v2\0";
const ORDERED_ROW_DOMAIN: &[u8] = b"brutex.cli.execution-disposition-order.v2\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex.cli.execution-disposition-completion.v2\0";

/// Fixed byte width of one sealed Population V4 row disposition.
pub const EXECUTION_DISPOSITION_ROW_STRIDE_V2: usize = ROW_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed byte width of one sealed V2 completion receipt.
pub const EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2: usize = COMPLETION_PAYLOAD_BYTES + SEAL_BYTES;
/// Maximum number of V2 rows returned by one page call.
pub const MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2: u64 = 256;

const _: () = assert!(ROW_PAYLOAD_BYTES == 480);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES == 512);
const _: () = assert!(EXECUTION_DISPOSITION_ROW_STRIDE_V2 == 512);
const _: () = assert!(EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2 == 544);
const _: () = assert!(MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2 <= MAX_ADMISSION_PAGE_ROWS_V1);

/// Stable terminal execution classification of one complete population row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionDispositionTagV2 {
    /// The policy admitted this exact coordinate and a sparse capability exists.
    Authorized,
    /// The coordinate was structurally valid but failed measured execution policy.
    PolicyRefused,
}

/// Freshly proven terminal corresponding to one durable V2 row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReclassifiedExecutionDispositionV2 {
    /// Exact sparse capability recreated from the fresh selected exit.
    Authorized(Box<ExecutionStrategyCapabilityV1>),
    /// Exact non-empty refusal bitmap recreated from the fresh complete grid.
    PolicyRefused(ExecutionRefusalBitsV1),
}

impl ExecutionDispositionTagV2 {
    const fn byte(self) -> u8 {
        match self {
            Self::Authorized => 1,
            Self::PolicyRefused => 2,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, ExecutionDispositionRefusalV2> {
        match byte {
            1 => Ok(Self::Authorized),
            2 => Ok(Self::PolicyRefused),
            _ => Err(format!(
                "execution disposition tag {byte} is unknown; V2 defines only 1 and 2"
            )),
        }
    }

    const fn matrix_column(self) -> usize {
        match self {
            Self::Authorized => 0,
            Self::PolicyRefused => 1,
        }
    }
}

/// One exact Population V4 row's sealed execution outcome.
///
/// All fields are private.  New records can be minted only from the opaque
/// runner classification and, for `Authorized`, the exact selected capability.
/// Decoding revalidates every redundant tag/bit/capability relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionRowDispositionV2 {
    disposition_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    row_payload_digest: [u8; 32],
    strategy_digest: [u8; 32],
    capability_id: [u8; 32],
    parameter_id: [u8; 32],
    runner_disposition_digest: [u8; 32],
    resolution_digest: [u8; 32],
    training_run_id: [u8; 32],
    context_digest: [u8; 32],
    column_digest: [u8; 32],
    row_sequence: u64,
    refusal_bits: ExecutionRefusalBitsV1,
    admission_status: AdmissionStatusV1,
    tag: ExecutionDispositionTagV2,
    coordinate: Chosen,
}

impl ExecutionRowDispositionV2 {
    /// Seals one opaque runner classification against exact population and
    /// admission authorities.
    ///
    /// `Authorized` requires the one capability minted from the same selected
    /// exit. `PolicyRefused` requires no capability and retains the runner's
    /// exact non-empty refusal bits. Both terminals must carry the exact row
    /// coordinate, side, mask, horizon, evaluator fingerprint, resolution, run,
    /// column and complete evaluated-grid context of the matching side-specific
    /// parameter authority. Structural/foreign-coordinate errors never produce
    /// an [`ExecutionDispositionV1`] and therefore cannot enter here.
    ///
    /// # Errors
    ///
    /// Refuses any population/V4/admission/row mismatch, a forged legacy
    /// admission summary, missing or extra capability, selected-exit mismatch,
    /// or malformed runner classification.
    pub fn from_classification(
        population_v4: &CompletionReceiptV4,
        admission_completion: &AdmissionCompletionReceiptV1,
        row: PopulationRowV1,
        admission_decision: &AdmissionDecisionRecordV1,
        parameters: &ExecutionParametersV1,
        classification: &ExecutionDispositionV1,
        capability: Option<ExecutionStrategyCapabilityV1>,
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        let population_v4_digest = population_v4
            .content_digest()
            .map_err(|why| format!("Population V4 digest could not be derived: {why}"))?;
        let admission_completion_digest = admission_completion
            .digest()
            .map_err(|why| format!("admission completion digest could not be derived: {why}"))?;
        validate_joined_row(
            population_v4,
            population_v4_digest,
            admission_completion,
            admission_completion_digest,
            &row,
            admission_decision,
        )?;
        validate_parameter_for_row(population_v4, population_v4_digest, parameters, &row)?;
        validate_classification_for_row(population_v4, parameters, &row, classification)?;

        let (tag, refusal_bits, capability_id) = if classification.is_authorized() {
            let selected = classification.selected().ok_or_else(|| {
                "authorized execution disposition carries no selected exit".to_owned()
            })?;
            if !classification.refusal_bits().is_empty() {
                return Err(
                    "authorized execution disposition carries policy-refusal bits".to_owned(),
                );
            }
            let capability = capability.ok_or_else(|| {
                "authorized execution disposition has no sparse row capability".to_owned()
            })?;
            if capability.parameter_id() != parameters.parameter_id()
                || capability.population_id() != row.population_id
                || capability.row_sequence() != row.sequence
                || capability.training_run_id() != classification.run_id().bytes()
                || capability.selected_exit_digest() != selected.digest()
            {
                return Err(
                    "authorized sparse capability differs from its classified population row"
                        .to_owned(),
                );
            }
            (
                ExecutionDispositionTagV2::Authorized,
                ExecutionRefusalBitsV1::NONE,
                capability.capability_id(),
            )
        } else if classification.is_policy_refused() {
            if classification.selected().is_some() {
                return Err(
                    "policy-refused execution disposition carries a selected exit".to_owned(),
                );
            }
            if capability.is_some() {
                return Err(
                    "policy-refused execution disposition carries an authorized capability"
                        .to_owned(),
                );
            }
            let bits = classification.refusal_bits();
            if bits.is_empty() {
                return Err("policy-refused execution disposition has no reason bit".to_owned());
            }
            (ExecutionDispositionTagV2::PolicyRefused, bits, [0_u8; 32])
        } else {
            return Err(
                "runner execution disposition is neither authorized nor policy-refused".to_owned(),
            );
        };

        let mut out = Self {
            disposition_id: [0; 32],
            population_id: row.population_id,
            population_v4_digest,
            admission_completion_digest,
            row_payload_digest: row.payload_digest()?,
            strategy_digest: row.strategy_digest,
            capability_id,
            parameter_id: parameters.parameter_id(),
            runner_disposition_digest: classification.disposition_digest(),
            resolution_digest: classification.resolution_digest(),
            training_run_id: classification.run_id().bytes(),
            context_digest: classification.context_digest(),
            column_digest: classification.column_digest(),
            row_sequence: row.sequence,
            refusal_bits,
            admission_status: admission_decision.verdict().status(),
            tag,
            coordinate: classification.coordinate(),
        };
        out.disposition_id = out.derived_id()?;
        out.validate()?;
        Ok(out)
    }

    /// Stable identity of this exact execution outcome and all upstream joins.
    #[must_use]
    pub const fn disposition_id(self) -> [u8; 32] {
        self.disposition_id
    }

    /// Alias naming the row-level execution authority used by triple joins.
    #[must_use]
    pub const fn execution_authority_id(self) -> [u8; 32] {
        self.disposition_id
    }

    /// Source Population V4 identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Digest of the exact Population V4 completion payload.
    #[must_use]
    pub const fn population_v4_digest(self) -> [u8; 32] {
        self.population_v4_digest
    }

    /// Digest of the exact recomputed admission completion.
    #[must_use]
    pub const fn admission_completion_digest(self) -> [u8; 32] {
        self.admission_completion_digest
    }

    /// Exact zero-based Population V4 row sequence.
    #[must_use]
    pub const fn row_sequence(self) -> u64 {
        self.row_sequence
    }

    /// Digest of the exact canonical population-row payload.
    #[must_use]
    pub const fn row_payload_digest(self) -> [u8; 32] {
        self.row_payload_digest
    }

    /// Exact durable strategy identity from the Population V4 row.
    #[must_use]
    pub const fn strategy_digest(self) -> [u8; 32] {
        self.strategy_digest
    }

    /// Side-specific dynamic execution-parameter authority for this row.
    #[must_use]
    pub const fn parameter_id(self) -> [u8; 32] {
        self.parameter_id
    }

    /// Opaque runner identity of the exact terminal classification.
    #[must_use]
    pub const fn runner_disposition_digest(self) -> [u8; 32] {
        self.runner_disposition_digest
    }

    /// Exact TRAINING exit-grid resolution identity.
    #[must_use]
    pub const fn resolution_digest(self) -> [u8; 32] {
        self.resolution_digest
    }

    /// Exact execution run which evaluated the row's cell.
    #[must_use]
    pub const fn training_run_id(self) -> [u8; 32] {
        self.training_run_id
    }

    /// Complete evaluated-grid and cell context identity from the runner.
    #[must_use]
    pub const fn context_digest(self) -> [u8; 32] {
        self.context_digest
    }

    /// Exact condition-column identity used to evaluate the cell.
    #[must_use]
    pub const fn column_digest(self) -> [u8; 32] {
        self.column_digest
    }

    /// Canonical exit coordinate proven equal to the Population V4 row.
    #[must_use]
    pub const fn coordinate(self) -> Chosen {
        self.coordinate
    }

    /// Recomputed admission terminal status used by the count matrix.
    #[must_use]
    pub const fn admission_status(self) -> AdmissionStatusV1 {
        self.admission_status
    }

    /// Authorized versus policy-refused terminal execution tag.
    #[must_use]
    pub const fn tag(self) -> ExecutionDispositionTagV2 {
        self.tag
    }

    /// Exact runner refusal bits; empty for an authorized row.
    #[must_use]
    pub const fn refusal_bits(self) -> ExecutionRefusalBitsV1 {
        self.refusal_bits
    }

    /// Sparse selected-capability identity, present only for authorization.
    #[must_use]
    pub fn capability_id(self) -> Option<[u8; 32]> {
        match self.tag {
            ExecutionDispositionTagV2::Authorized => Some(self.capability_id),
            ExecutionDispositionTagV2::PolicyRefused => None,
        }
    }

    /// Whether this exact row may enter execution reconstruction.
    #[must_use]
    pub const fn is_authorized(self) -> bool {
        matches!(self.tag, ExecutionDispositionTagV2::Authorized)
    }

    /// Whether this structurally valid row was refused by execution policy.
    #[must_use]
    pub const fn is_policy_refused(self) -> bool {
        matches!(self.tag, ExecutionDispositionTagV2::PolicyRefused)
    }

    /// Reclassifies and proves this durable row against fresh runner evidence.
    ///
    /// The method rechecks the Population V4 row, the exact admission
    /// completion and decision, its side-specific dynamic parameters,
    /// canonical strategy/run identity, coordinate, resolution, mask, horizon,
    /// evaluator fingerprint, column, evaluated-grid context, terminal digest,
    /// tag and refusal bits. Authorization recreates the sparse
    /// [`ExecutionStrategyCapabilityV1`] and requires its identity to equal the
    /// stored capability. Policy refusal proves the stored row has no capability
    /// and carries exactly the fresh non-empty refusal bitmap.
    ///
    /// # Errors
    ///
    /// Refuses every cross-population, cross-admission-completion,
    /// changed-admission-status, cross-run, cross-side, cross-fingerprint,
    /// changed-grid/context, terminal-substitution or capability mismatch.
    pub fn require_reclassification(
        self,
        population_v4: &CompletionReceiptV4,
        admission_completion: &AdmissionCompletionReceiptV1,
        row: PopulationRowV1,
        admission_decision: &AdmissionDecisionRecordV1,
        parameters: &ExecutionParametersV1,
        classification: &ExecutionDispositionV1,
    ) -> Result<ReclassifiedExecutionDispositionV2, ExecutionDispositionRefusalV2> {
        self.validate()?;
        let population_v4_digest = population_v4
            .content_digest()
            .map_err(|why| format!("Population V4 digest could not be derived: {why}"))?;
        let admission_completion_digest = admission_completion
            .digest()
            .map_err(|why| format!("admission completion digest could not be derived: {why}"))?;
        validate_joined_row(
            population_v4,
            population_v4_digest,
            admission_completion,
            admission_completion_digest,
            &row,
            admission_decision,
        )?;
        if self.population_id != row.population_id
            || self.population_v4_digest != population_v4_digest
            || self.admission_completion_digest != admission_completion_digest
            || self.row_sequence != row.sequence
            || self.row_payload_digest != row.payload_digest()?
            || self.strategy_digest != row.strategy_digest
            || self.admission_status != admission_decision.verdict().status()
        {
            return Err(
                "durable execution disposition differs from its Population V4/admission row"
                    .to_owned(),
            );
        }
        validate_parameter_for_row(population_v4, population_v4_digest, parameters, &row)?;
        validate_classification_for_row(population_v4, parameters, &row, classification)?;
        if self.parameter_id != parameters.parameter_id()
            || self.runner_disposition_digest != classification.disposition_digest()
            || self.resolution_digest != classification.resolution_digest()
            || self.training_run_id != classification.run_id().bytes()
            || self.context_digest != classification.context_digest()
            || self.column_digest != classification.column_digest()
            || self.coordinate != classification.coordinate()
        {
            return Err(
                "fresh execution classification differs from durable V2 provenance".to_owned(),
            );
        }
        match self.tag {
            ExecutionDispositionTagV2::Authorized => {
                if !classification.is_authorized() || !classification.refusal_bits().is_empty() {
                    return Err(
                        "authorized durable row was replaced by a policy refusal".to_owned()
                    );
                }
                let selected = classification.selected().ok_or_else(|| {
                    "authorized fresh classification has no selected exit".to_owned()
                })?;
                let capability = ExecutionStrategyCapabilityV1::new(parameters, row, selected)?;
                if self.capability_id() != Some(capability.capability_id()) {
                    return Err(
                        "fresh authorized capability differs from durable V2 capability".to_owned(),
                    );
                }
                Ok(ReclassifiedExecutionDispositionV2::Authorized(Box::new(
                    capability,
                )))
            }
            ExecutionDispositionTagV2::PolicyRefused => {
                if !classification.is_policy_refused() || classification.selected().is_some() {
                    return Err(
                        "policy-refused durable row was replaced by authorization".to_owned()
                    );
                }
                let bits = classification.refusal_bits();
                if bits.is_empty() || bits != self.refusal_bits || self.capability_id().is_some() {
                    return Err(
                        "fresh policy refusal differs from durable V2 refusal/capability state"
                            .to_owned(),
                    );
                }
                Ok(ReclassifiedExecutionDispositionV2::PolicyRefused(bits))
            }
        }
    }

    fn validate(self) -> Result<(), ExecutionDispositionRefusalV2> {
        for (name, digest) in [
            ("execution disposition identity", self.disposition_id),
            ("execution disposition population", self.population_id),
            (
                "execution disposition Population V4 digest",
                self.population_v4_digest,
            ),
            (
                "execution disposition admission completion",
                self.admission_completion_digest,
            ),
            ("execution disposition row payload", self.row_payload_digest),
            ("execution disposition strategy", self.strategy_digest),
            ("execution disposition parameter", self.parameter_id),
            (
                "runner terminal disposition identity",
                self.runner_disposition_digest,
            ),
            ("execution disposition resolution", self.resolution_digest),
            ("execution disposition training run", self.training_run_id),
            ("execution disposition context", self.context_digest),
            ("execution disposition column", self.column_digest),
        ] {
            require_digest(name, &digest)?;
        }
        match self.tag {
            ExecutionDispositionTagV2::Authorized => {
                require_digest("authorized sparse capability", &self.capability_id)?;
                if !self.refusal_bits.is_empty() {
                    return Err("authorized execution disposition carries refusal bits".to_owned());
                }
            }
            ExecutionDispositionTagV2::PolicyRefused => {
                if self.capability_id != [0; 32] {
                    return Err(
                        "policy-refused execution disposition carries a capability identity"
                            .to_owned(),
                    );
                }
                if self.refusal_bits.is_empty() {
                    return Err("policy-refused execution disposition has no reason bit".to_owned());
                }
            }
        }
        validate_chosen(self.coordinate)?;
        if self.disposition_id != self.derived_id()? {
            return Err("execution disposition identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_id(self) -> Result<[u8; 32], ExecutionDispositionRefusalV2> {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(ROW_ID_DOMAIN);
        for digest in [
            self.population_id,
            self.population_v4_digest,
            self.admission_completion_digest,
            self.row_payload_digest,
            self.strategy_digest,
            self.capability_id,
            self.parameter_id,
            self.runner_disposition_digest,
            self.resolution_digest,
            self.training_run_id,
            self.context_digest,
            self.column_digest,
        ] {
            hasher.update(&digest);
        }
        hasher.update(&self.row_sequence.to_le_bytes());
        hasher.update(&self.refusal_bits.bits().to_le_bytes());
        hasher.update(&[admission_status_byte(self.admission_status)]);
        hasher.update(&[self.tag.byte()]);
        hash_chosen(&mut hasher, self.coordinate)?;
        hasher.update(&exact_execution_law_digest_v1());
        Ok(hasher.finalize())
    }

    fn payload(self) -> Result<[u8; ROW_PAYLOAD_BYTES], ExecutionDispositionRefusalV2> {
        self.validate()?;
        let mut raw = [0; ROW_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        for digest in [
            self.disposition_id,
            self.population_id,
            self.population_v4_digest,
            self.admission_completion_digest,
            self.row_payload_digest,
            self.strategy_digest,
            self.capability_id,
            self.parameter_id,
            self.runner_disposition_digest,
            self.resolution_digest,
            self.training_run_id,
            self.context_digest,
            self.column_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.row_sequence)?;
        encoder.u64(self.refusal_bits.bits())?;
        encoder.u8(admission_status_byte(self.admission_status))?;
        encoder.u8(self.tag.byte())?;
        encode_chosen(&mut encoder, self.coordinate)?;
        encoder.zeros(6)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(
        self,
    ) -> Result<[u8; EXECUTION_DISPOSITION_ROW_STRIDE_V2], ExecutionDispositionRefusalV2> {
        with_seal(self.payload()?)
    }

    fn from_bytes(
        raw: &[u8; EXECUTION_DISPOSITION_ROW_STRIDE_V2],
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        let payload = checked_payload::<ROW_PAYLOAD_BYTES, EXECUTION_DISPOSITION_ROW_STRIDE_V2>(
            raw,
            "execution disposition row",
        )?;
        let mut decoder = Decoder::new(&payload);
        let disposition = Self {
            disposition_id: decoder.array_32()?,
            population_id: decoder.array_32()?,
            population_v4_digest: decoder.array_32()?,
            admission_completion_digest: decoder.array_32()?,
            row_payload_digest: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            capability_id: decoder.array_32()?,
            parameter_id: decoder.array_32()?,
            runner_disposition_digest: decoder.array_32()?,
            resolution_digest: decoder.array_32()?,
            training_run_id: decoder.array_32()?,
            context_digest: decoder.array_32()?,
            column_digest: decoder.array_32()?,
            row_sequence: decoder.u64()?,
            refusal_bits: ExecutionRefusalBitsV1::from_bits(decoder.u64()?).ok_or_else(|| {
                "execution disposition contains unknown policy-refusal bits".to_owned()
            })?,
            admission_status: admission_status_from_byte(decoder.u8()?)?,
            tag: ExecutionDispositionTagV2::from_byte(decoder.u8()?)?,
            coordinate: decode_chosen(&mut decoder)?,
        };
        decoder.zeros(6, "execution disposition row reserve")?;
        decoder.finish()?;
        disposition.validate()?;
        Ok(disposition)
    }
}

/// Fixed four-by-two count matrix over admission and execution terminals.
///
/// Row order is Admitted, Rejected, Unmeasured, Refused. Column order is
/// `Authorized`, `PolicyRefused`. The layout is public through checked accessors,
/// not through a caller-maintained index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionExecutionMatrixV2 {
    counts: [u64; 8],
}

impl AdmissionExecutionMatrixV2 {
    const ZERO: Self = Self { counts: [0; 8] };

    /// Exact count at one admission/execution intersection.
    #[must_use]
    pub const fn count(
        self,
        admission: AdmissionStatusV1,
        execution: ExecutionDispositionTagV2,
    ) -> u64 {
        let [
            admitted_authorized,
            admitted_refused,
            rejected_authorized,
            rejected_refused,
            unmeasured_authorized,
            unmeasured_refused,
            refused_authorized,
            refused_refused,
        ] = self.counts;
        match (admission, execution) {
            (AdmissionStatusV1::Admitted, ExecutionDispositionTagV2::Authorized) => {
                admitted_authorized
            }
            (AdmissionStatusV1::Admitted, ExecutionDispositionTagV2::PolicyRefused) => {
                admitted_refused
            }
            (AdmissionStatusV1::Rejected, ExecutionDispositionTagV2::Authorized) => {
                rejected_authorized
            }
            (AdmissionStatusV1::Rejected, ExecutionDispositionTagV2::PolicyRefused) => {
                rejected_refused
            }
            (AdmissionStatusV1::Unmeasured, ExecutionDispositionTagV2::Authorized) => {
                unmeasured_authorized
            }
            (AdmissionStatusV1::Unmeasured, ExecutionDispositionTagV2::PolicyRefused) => {
                unmeasured_refused
            }
            (AdmissionStatusV1::Refused, ExecutionDispositionTagV2::Authorized) => {
                refused_authorized
            }
            (AdmissionStatusV1::Refused, ExecutionDispositionTagV2::PolicyRefused) => {
                refused_refused
            }
        }
    }

    /// Stable row-major bytes exposed for comparison/audit surfaces.
    #[must_use]
    pub const fn counts(self) -> [u64; 8] {
        self.counts
    }

    fn push(
        &mut self,
        admission: AdmissionStatusV1,
        execution: ExecutionDispositionTagV2,
    ) -> Result<(), ExecutionDispositionRefusalV2> {
        let index = matrix_index(admission, execution);
        let held = self
            .counts
            .get_mut(index)
            .ok_or_else(|| "execution count-matrix index is absent".to_owned())?;
        *held = held
            .checked_add(1)
            .ok_or_else(|| "execution count-matrix cell overflowed u64".to_owned())?;
        Ok(())
    }

    fn total(self) -> Result<u64, ExecutionDispositionRefusalV2> {
        checked_sum(&self.counts, "admission/execution matrix")
    }

    fn admission_total(
        self,
        admission: AdmissionStatusV1,
    ) -> Result<u64, ExecutionDispositionRefusalV2> {
        self.count(admission, ExecutionDispositionTagV2::Authorized)
            .checked_add(self.count(admission, ExecutionDispositionTagV2::PolicyRefused))
            .ok_or_else(|| "admission/execution matrix admission row overflowed u64".to_owned())
    }

    fn execution_total(
        self,
        execution: ExecutionDispositionTagV2,
    ) -> Result<u64, ExecutionDispositionRefusalV2> {
        [
            AdmissionStatusV1::Admitted,
            AdmissionStatusV1::Rejected,
            AdmissionStatusV1::Unmeasured,
            AdmissionStatusV1::Refused,
        ]
        .into_iter()
        .try_fold(0_u64, |sum, admission| {
            sum.checked_add(self.count(admission, execution))
                .ok_or_else(|| {
                    "admission/execution matrix execution column overflowed u64".to_owned()
                })
        })
    }
}

const fn matrix_index(admission: AdmissionStatusV1, execution: ExecutionDispositionTagV2) -> usize {
    admission_status_index(admission) * 2 + execution.matrix_column()
}

const fn admission_status_index(status: AdmissionStatusV1) -> usize {
    match status {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    }
}

const fn admission_status_byte(status: AdmissionStatusV1) -> u8 {
    match status {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    }
}

fn admission_status_from_byte(
    byte: u8,
) -> Result<AdmissionStatusV1, ExecutionDispositionRefusalV2> {
    match byte {
        0 => Ok(AdmissionStatusV1::Admitted),
        1 => Ok(AdmissionStatusV1::Rejected),
        2 => Ok(AdmissionStatusV1::Unmeasured),
        3 => Ok(AdmissionStatusV1::Refused),
        _ => Err(format!(
            "execution disposition admission-status tag {byte} is unknown"
        )),
    }
}

/// Receipt-last proof of one complete Population V4 execution disposition.
///
/// `row_first` is a physical append offset and is deliberately excluded from
/// [`Self::completion_id`]. A crash may leave valid orphan rows before an exact
/// retry; semantic identity must not depend on where the retry lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionCapabilityCompletionV2 {
    completion_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    long_parameter_id: [u8; 32],
    short_parameter_id: [u8; 32],
    ordered_parameter_digest: [u8; 32],
    ordered_percentile_digest: [u8; 32],
    ordered_disposition_digest: [u8; 32],
    execution_law_digest: [u8; 32],
    parameter_first: u64,
    parameter_count: u64,
    percentile_first: u64,
    percentile_count: u64,
    row_first: u64,
    disposition_count: u64,
    row_count: u64,
    authorized_capability_count: u64,
    policy_refused_count: u64,
    matrix: AdmissionExecutionMatrixV2,
}

impl ExecutionCapabilityCompletionV2 {
    /// Stable semantic completion identity, independent of physical offsets.
    #[must_use]
    pub const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Alias naming this complete execution authority.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.completion_id
    }

    /// Population V4 covered row-for-row by this completion.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Digest of the exact Population V4 completion payload.
    #[must_use]
    pub const fn population_v4_digest(self) -> [u8; 32] {
        self.population_v4_digest
    }

    /// Digest of the exact admission completion used by the matrix.
    #[must_use]
    pub const fn admission_completion_digest(self) -> [u8; 32] {
        self.admission_completion_digest
    }

    /// Exact long-side dynamic execution-parameter authority.
    #[must_use]
    pub const fn long_parameter_id(self) -> [u8; 32] {
        self.long_parameter_id
    }

    /// Exact short-side dynamic execution-parameter authority.
    #[must_use]
    pub const fn short_parameter_id(self) -> [u8; 32] {
        self.short_parameter_id
    }

    /// Canonical `[long, short]` parameter identities.
    #[must_use]
    pub const fn parameter_ids(self) -> [[u8; 32]; 2] {
        [self.long_parameter_id, self.short_parameter_id]
    }

    /// Digest over the exact two scalar parameter records in long/short order.
    #[must_use]
    pub const fn ordered_parameter_digest(self) -> [u8; 32] {
        self.ordered_parameter_digest
    }

    /// Digest over every runtime-sized percentile atom for both parameters.
    #[must_use]
    pub const fn ordered_percentile_digest(self) -> [u8; 32] {
        self.ordered_percentile_digest
    }

    /// Exact number of Population V4 rows and disposition records.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Number of sparse authorized capabilities.
    #[must_use]
    pub const fn authorized_capability_count(self) -> u64 {
        self.authorized_capability_count
    }

    /// Number of structurally valid rows refused by execution policy.
    #[must_use]
    pub const fn policy_refused_count(self) -> u64 {
        self.policy_refused_count
    }

    /// Complete admission-by-execution terminal count matrix.
    #[must_use]
    pub const fn admission_execution_matrix(self) -> AdmissionExecutionMatrixV2 {
        self.matrix
    }

    /// Digest over exact sealed row records in canonical row order.
    #[must_use]
    pub const fn ordered_disposition_digest(self) -> [u8; 32] {
        self.ordered_disposition_digest
    }

    /// Exact one-minute/15:10/printed-OHLCV law bound by this authority.
    #[must_use]
    pub const fn execution_law_digest(self) -> [u8; 32] {
        self.execution_law_digest
    }

    fn for_prepared(
        prepared: &PreparedExecutionDispositionsV2,
        parameter_first: u64,
        percentile_first: u64,
        row_first: u64,
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        let mut completion = Self {
            completion_id: [0; 32],
            population_id: prepared.population_id,
            population_v4_digest: prepared.population_v4_digest,
            admission_completion_digest: prepared.admission_completion_digest,
            long_parameter_id: prepared.parameters[0].parameter_id(),
            short_parameter_id: prepared.parameters[1].parameter_id(),
            ordered_parameter_digest: prepared.ordered_parameter_digest,
            ordered_percentile_digest: prepared.ordered_percentile_digest,
            ordered_disposition_digest: prepared.ordered_disposition_digest,
            execution_law_digest: exact_execution_law_digest_v1(),
            parameter_first,
            parameter_count: usize_u64(prepared.scalars.len(), "execution parameter block")?,
            percentile_first,
            percentile_count: usize_u64(prepared.percentiles.len(), "execution percentile block")?,
            row_first,
            disposition_count: prepared.row_count,
            row_count: prepared.row_count,
            authorized_capability_count: prepared.authorized_capability_count,
            policy_refused_count: prepared.policy_refused_count,
            matrix: prepared.matrix,
        };
        completion.completion_id = completion.derived_id();
        completion.validate()?;
        Ok(completion)
    }

    fn validate(self) -> Result<(), ExecutionDispositionRefusalV2> {
        for (name, digest) in [
            ("execution V2 completion identity", self.completion_id),
            ("execution V2 population identity", self.population_id),
            (
                "execution V2 Population V4 digest",
                self.population_v4_digest,
            ),
            (
                "execution V2 admission completion",
                self.admission_completion_digest,
            ),
            ("execution V2 long parameter", self.long_parameter_id),
            ("execution V2 short parameter", self.short_parameter_id),
            (
                "execution V2 ordered parameter digest",
                self.ordered_parameter_digest,
            ),
            (
                "execution V2 ordered percentile digest",
                self.ordered_percentile_digest,
            ),
            (
                "execution V2 ordered disposition digest",
                self.ordered_disposition_digest,
            ),
            ("execution V2 law digest", self.execution_law_digest),
        ] {
            require_digest(name, &digest)?;
        }
        if self.long_parameter_id == self.short_parameter_id {
            return Err(
                "execution V2 long and short parameter identities are identical".to_owned(),
            );
        }
        if self.parameter_count != 2 {
            return Err(format!(
                "execution V2 parameter block has {}, not exact long+short count 2",
                self.parameter_count
            ));
        }
        self.parameter_first
            .checked_add(self.parameter_count)
            .ok_or_else(|| "execution V2 parameter block range overflows u64".to_owned())?;
        self.percentile_first
            .checked_add(self.percentile_count)
            .ok_or_else(|| "execution V2 percentile block range overflows u64".to_owned())?;
        self.row_first
            .checked_add(self.disposition_count)
            .ok_or_else(|| "execution V2 disposition block range overflows u64".to_owned())?;
        if self.disposition_count != self.row_count {
            return Err(
                "execution V2 disposition count differs from Population V4 row count".to_owned(),
            );
        }
        let classified = self
            .authorized_capability_count
            .checked_add(self.policy_refused_count)
            .ok_or_else(|| "execution V2 terminal counts overflow u64".to_owned())?;
        if classified != self.row_count {
            return Err(format!(
                "execution V2 authorized plus policy-refused count is {classified}, not row_count {}",
                self.row_count
            ));
        }
        if self.matrix.total()? != self.row_count {
            return Err(
                "execution V2 admission/execution matrix does not total row_count".to_owned(),
            );
        }
        if self
            .matrix
            .execution_total(ExecutionDispositionTagV2::Authorized)?
            != self.authorized_capability_count
            || self
                .matrix
                .execution_total(ExecutionDispositionTagV2::PolicyRefused)?
                != self.policy_refused_count
        {
            return Err(
                "execution V2 matrix columns differ from terminal execution counts".to_owned(),
            );
        }
        if self.execution_law_digest != exact_execution_law_digest_v1() {
            return Err(
                "execution V2 completion does not bind the exact one-minute/15:10 law".to_owned(),
            );
        }
        if self.completion_id != self.derived_id() {
            return Err("execution V2 completion identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn require_admission_marginals(
        self,
        admission: &AdmissionCompletionReceiptV1,
    ) -> Result<(), ExecutionDispositionRefusalV2> {
        for (status, expected) in [
            (AdmissionStatusV1::Admitted, admission.admitted_count()),
            (AdmissionStatusV1::Rejected, admission.rejected_count()),
            (AdmissionStatusV1::Unmeasured, admission.unmeasured_count()),
            (AdmissionStatusV1::Refused, admission.refused_count()),
        ] {
            let actual = self.matrix.admission_total(status)?;
            if actual != expected {
                return Err(format!(
                    "execution V2 matrix {status:?} admission total is {actual}, not authoritative {expected}"
                ));
            }
        }
        Ok(())
    }

    fn derived_id(self) -> [u8; 32] {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        for digest in [
            self.population_id,
            self.population_v4_digest,
            self.admission_completion_digest,
            self.long_parameter_id,
            self.short_parameter_id,
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_disposition_digest,
            self.execution_law_digest,
        ] {
            hasher.update(&digest);
        }
        for count in [
            self.parameter_count,
            self.percentile_count,
            self.disposition_count,
            self.row_count,
            self.authorized_capability_count,
            self.policy_refused_count,
        ] {
            hasher.update(&count.to_le_bytes());
        }
        for count in self.matrix.counts {
            hasher.update(&count.to_le_bytes());
        }
        hasher.finalize()
    }

    fn same_semantics(self, prepared: &PreparedExecutionDispositionsV2) -> bool {
        Self::for_prepared(
            prepared,
            self.parameter_first,
            self.percentile_first,
            self.row_first,
        )
        .is_ok_and(|candidate| candidate == self)
    }

    fn payload(self) -> Result<[u8; COMPLETION_PAYLOAD_BYTES], ExecutionDispositionRefusalV2> {
        self.validate()?;
        let mut raw = [0; COMPLETION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        for digest in [
            self.completion_id,
            self.population_id,
            self.population_v4_digest,
            self.admission_completion_digest,
            self.long_parameter_id,
            self.short_parameter_id,
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_disposition_digest,
            self.execution_law_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        for value in [
            self.parameter_first,
            self.parameter_count,
            self.percentile_first,
            self.percentile_count,
            self.row_first,
            self.disposition_count,
            self.row_count,
            self.authorized_capability_count,
            self.policy_refused_count,
        ] {
            encoder.u64(value)?;
        }
        for value in self.matrix.counts {
            encoder.u64(value)?;
        }
        encoder.zeros(56)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(
        self,
    ) -> Result<[u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2], ExecutionDispositionRefusalV2>
    {
        with_seal(self.payload()?)
    }

    fn from_bytes(
        raw: &[u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2],
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        let payload = checked_payload::<
            COMPLETION_PAYLOAD_BYTES,
            EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2,
        >(raw, "execution V2 completion")?;
        let mut decoder = Decoder::new(&payload);
        let completion_id = decoder.array_32()?;
        let population_id = decoder.array_32()?;
        let population_v4_digest = decoder.array_32()?;
        let admission_completion_digest = decoder.array_32()?;
        let long_parameter_id = decoder.array_32()?;
        let short_parameter_id = decoder.array_32()?;
        let ordered_parameter_digest = decoder.array_32()?;
        let ordered_percentile_digest = decoder.array_32()?;
        let ordered_disposition_digest = decoder.array_32()?;
        let execution_law_digest = decoder.array_32()?;
        let parameter_first = decoder.u64()?;
        let parameter_count = decoder.u64()?;
        let percentile_first = decoder.u64()?;
        let percentile_count = decoder.u64()?;
        let row_first = decoder.u64()?;
        let disposition_count = decoder.u64()?;
        let row_count = decoder.u64()?;
        let authorized_capability_count = decoder.u64()?;
        let policy_refused_count = decoder.u64()?;
        let mut counts = [0_u64; 8];
        for count in &mut counts {
            *count = decoder.u64()?;
        }
        decoder.zeros(56, "execution V2 completion reserve")?;
        decoder.finish()?;
        let completion = Self {
            completion_id,
            population_id,
            population_v4_digest,
            admission_completion_digest,
            long_parameter_id,
            short_parameter_id,
            ordered_parameter_digest,
            ordered_percentile_digest,
            ordered_disposition_digest,
            execution_law_digest,
            parameter_first,
            parameter_count,
            percentile_first,
            percentile_count,
            row_first,
            disposition_count,
            row_count,
            authorized_capability_count,
            policy_refused_count,
            matrix: AdmissionExecutionMatrixV2 { counts },
        };
        completion.validate()?;
        Ok(completion)
    }
}

/// Fully reconciled rows ready for receipt-last persistence.
#[derive(Clone, Debug)]
pub struct PreparedExecutionDispositionsV2 {
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    parameters: [ExecutionParametersV1; 2],
    scalars: [ParameterScalarV1; 2],
    percentiles: Vec<PercentileRecordV1>,
    ordered_parameter_digest: [u8; 32],
    ordered_percentile_digest: [u8; 32],
    row_count: u64,
    authorized_capability_count: u64,
    policy_refused_count: u64,
    matrix: AdmissionExecutionMatrixV2,
    ordered_disposition_digest: [u8; 32],
    rows: Vec<ExecutionRowDispositionV2>,
}

impl PreparedExecutionDispositionsV2 {
    /// Reconciles one disposition per exact Population V4/admission row.
    ///
    /// Both source ledgers are read in bounded pages. The population's legacy
    /// admission summary is compared with the recomputed admission verdict and
    /// can never authorize the matrix on its own.
    ///
    /// # Errors
    ///
    /// Refuses absent/stale authorities, row count/order/identity drift,
    /// duplicate dispositions, terminal tag/bit/capability mismatch, changed
    /// admission marginal counts, or any bounded source-page refusal.
    #[expect(
        clippy::too_many_lines,
        reason = "the bounded three-authority reconciliation keeps each fail-closed comparison adjacent to the record it admits"
    )]
    pub fn from_ledgers(
        population: &mut PopulationLedger,
        admission: &mut AdmissionAuthorityLedger,
        population_id: [u8; 32],
        long_parameters: ExecutionParametersV1,
        short_parameters: ExecutionParametersV1,
        rows: Vec<ExecutionRowDispositionV2>,
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        let population_v4 = population
            .receipt_v4(&population_id)
            .ok_or_else(|| "execution V2 preparation requires Population V4".to_owned())?;
        let population_v4_digest = population_v4
            .content_digest()
            .map_err(|why| format!("Population V4 digest could not be derived: {why}"))?;
        let admission_completion = admission
            .completion(population_id)?
            .ok_or_else(|| "execution V2 preparation requires admission completion".to_owned())?;
        let admission_completion_digest = admission_completion.digest()?;
        if admission_completion.population_v4_completion_digest() != population_v4_digest {
            return Err(
                "execution V2 admission completion names another Population V4 digest".to_owned(),
            );
        }
        let expected_rows = population_v4.v3().v2().row_count;
        if admission_completion.decision_count() != expected_rows {
            return Err(
                "execution V2 admission decision count differs from Population V4 row_count"
                    .to_owned(),
            );
        }
        validate_parameter_pair_for_population(
            &population_v4,
            population_v4_digest,
            &long_parameters,
            &short_parameters,
        )?;
        let parameters = [long_parameters, short_parameters];
        let scalars = [
            ParameterScalarV1::from_parameters(&parameters[0])?,
            ParameterScalarV1::from_parameters(&parameters[1])?,
        ];
        let mut percentiles = PercentileRecordV1::from_parameters(&parameters[0])?;
        let short_percentiles = PercentileRecordV1::from_parameters(&parameters[1])?;
        percentiles
            .try_reserve(short_percentiles.len())
            .map_err(|why| format!("execution V2 percentile allocation refused: {why}"))?;
        percentiles.extend(short_percentiles);
        let ordered_parameter_digest = parameter_digest_records(&scalars)?;
        let ordered_percentile_digest = percentile_digest_records(&percentiles)?;
        let supplied_rows = u64::try_from(rows.len())
            .map_err(|_| "execution V2 row count does not fit u64".to_owned())?;
        if supplied_rows != expected_rows {
            return Err(format!(
                "execution V2 received {supplied_rows} dispositions for {expected_rows} Population V4 rows"
            ));
        }

        let mut matrix = AdmissionExecutionMatrixV2::ZERO;
        let mut authorized_capability_count = 0_u64;
        let mut policy_refused_count = 0_u64;
        let mut ordered = brutex_core::blake3::Hasher::new();
        ordered.update(ORDERED_ROW_DOMAIN);
        ordered.update(&expected_rows.to_le_bytes());
        let capacity = usize::try_from(expected_rows)
            .map_err(|_| "execution V2 uniqueness count does not fit usize".to_owned())?;
        let mut identities = HashSet::new();
        identities.try_reserve(capacity).map_err(|why| {
            format!("execution V2 identity set could not reserve {capacity} rows: {why}")
        })?;

        let mut offset = 0_u64;
        while offset < expected_rows {
            let limit = expected_rows
                .saturating_sub(offset)
                .min(MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2);
            let population_page = population
                .page_v4(&population_id, offset, limit)?
                .ok_or_else(|| {
                    "Population V4 disappeared during execution V2 preparation".to_owned()
                })?;
            let admission_page =
                admission
                    .page(population_id, offset, limit)?
                    .ok_or_else(|| {
                        "admission completion disappeared during execution V2 preparation"
                            .to_owned()
                    })?;
            let count = usize::try_from(limit)
                .map_err(|_| "bounded execution V2 page count does not fit usize".to_owned())?;
            if population_page.rows.len() != count || admission_page.len() != count {
                return Err(
                    "execution V2 source page did not contain its complete requested row count"
                        .to_owned(),
                );
            }
            let start = usize::try_from(offset)
                .map_err(|_| "execution V2 page offset does not fit usize".to_owned())?;
            let end = start
                .checked_add(count)
                .ok_or_else(|| "execution V2 page range overflowed usize".to_owned())?;
            let supplied = rows
                .get(start..end)
                .ok_or_else(|| "execution V2 supplied page range is absent".to_owned())?;

            for ((disposition, population_row), admission_decision) in supplied
                .iter()
                .zip(&population_page.rows)
                .zip(&admission_page)
            {
                validate_reopened_join(
                    disposition,
                    population_v4_digest,
                    admission_completion_digest,
                    population_row,
                    admission_decision,
                )?;
                let parameters = match population_row.direction {
                    TradeDirectionV1::Long => &parameters[0],
                    TradeDirectionV1::Short => &parameters[1],
                };
                validate_reopened_parameter_join(disposition, population_row, parameters)?;
                if !identities.insert(disposition.disposition_id) {
                    return Err("execution V2 repeats a disposition identity".to_owned());
                }
                matrix.push(disposition.admission_status, disposition.tag)?;
                match disposition.tag {
                    ExecutionDispositionTagV2::Authorized => {
                        authorized_capability_count =
                            authorized_capability_count.checked_add(1).ok_or_else(|| {
                                "authorized capability count overflowed u64".to_owned()
                            })?;
                    }
                    ExecutionDispositionTagV2::PolicyRefused => {
                        policy_refused_count = policy_refused_count
                            .checked_add(1)
                            .ok_or_else(|| "policy-refused count overflowed u64".to_owned())?;
                    }
                }
                ordered.update(&disposition.to_bytes()?);
            }
            offset = offset
                .checked_add(limit)
                .ok_or_else(|| "execution V2 page offset overflowed u64".to_owned())?;
        }

        let prepared = Self {
            population_id,
            population_v4_digest,
            admission_completion_digest,
            parameters,
            scalars,
            percentiles,
            ordered_parameter_digest,
            ordered_percentile_digest,
            row_count: expected_rows,
            authorized_capability_count,
            policy_refused_count,
            matrix,
            ordered_disposition_digest: ordered.finalize(),
            rows,
        };
        let completion = ExecutionCapabilityCompletionV2::for_prepared(&prepared, 0, 0, 0)?;
        completion.require_admission_marginals(&admission_completion)?;
        Ok(prepared)
    }

    /// Stable authority identity before physical append offsets are assigned.
    ///
    /// # Errors
    ///
    /// Refuses if the prepared parameter, percentile, row, matrix, or count
    /// authority cannot form a canonical semantic completion.
    pub fn completion_id(&self) -> Result<[u8; 32], ExecutionDispositionRefusalV2> {
        Ok(ExecutionCapabilityCompletionV2::for_prepared(self, 0, 0, 0)?.completion_id())
    }

    /// Exact reconciled row count.
    #[must_use]
    pub const fn row_count(&self) -> u64 {
        self.row_count
    }

    /// Exact sparse authorized-capability count.
    #[must_use]
    pub const fn authorized_capability_count(&self) -> u64 {
        self.authorized_capability_count
    }

    /// Exact policy-refused row count.
    #[must_use]
    pub const fn policy_refused_count(&self) -> u64 {
        self.policy_refused_count
    }

    /// Fully reconciled row records in Population V4 sequence order.
    #[must_use]
    pub fn rows(&self) -> &[ExecutionRowDispositionV2] {
        &self.rows
    }

    /// Exact canonical `[long, short]` dynamic parameter authorities.
    #[must_use]
    pub fn parameters(&self) -> &[ExecutionParametersV1; 2] {
        &self.parameters
    }
}

fn validate_joined_row(
    population_v4: &CompletionReceiptV4,
    population_v4_digest: [u8; 32],
    admission_completion: &AdmissionCompletionReceiptV1,
    admission_completion_digest: [u8; 32],
    row: &PopulationRowV1,
    decision: &AdmissionDecisionRecordV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let population_id = population_v4.population_id();
    if row.population_id != population_id
        || admission_completion.population_id() != population_id
        || decision.population_id() != population_id
    {
        return Err("execution V2 row joins different population identities".to_owned());
    }
    if admission_completion.population_v4_completion_digest() != population_v4_digest {
        return Err("execution V2 admission joins another Population V4 digest".to_owned());
    }
    if row.sequence >= admission_completion.decision_count() {
        return Err(
            "execution V2 admission decision lies outside its completion row count".to_owned(),
        );
    }
    if decision.row_sequence() != row.sequence
        || decision.strategy_digest() != row.strategy_digest
        || decision.row_payload_digest() != row.payload_digest()?
    {
        return Err("execution V2 admission decision differs from its population row".to_owned());
    }
    row.admission
        .require_matches_verdict(*decision.verdict())
        .map_err(|why| format!("execution V2 admission summary mismatch: {why}"))?;
    if admission_completion.policy().evaluate(decision.evidence()) != *decision.verdict() {
        return Err(
            "execution V2 admission decision does not recompute under its completion policy"
                .to_owned(),
        );
    }
    require_digest(
        "execution V2 admission completion digest",
        &admission_completion_digest,
    )
}

fn validate_reopened_join(
    disposition: &ExecutionRowDispositionV2,
    population_v4_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    row: &PopulationRowV1,
    decision: &AdmissionDecisionRecordV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    disposition.validate()?;
    if disposition.population_id != row.population_id
        || disposition.population_v4_digest != population_v4_digest
        || disposition.admission_completion_digest != admission_completion_digest
        || disposition.row_sequence != row.sequence
        || disposition.row_payload_digest != row.payload_digest()?
        || disposition.strategy_digest != row.strategy_digest
        || disposition.admission_status != decision.verdict().status()
        || decision.population_id() != row.population_id
        || decision.row_sequence() != row.sequence
        || decision.row_payload_digest() != disposition.row_payload_digest
        || decision.strategy_digest() != disposition.strategy_digest
    {
        return Err(
            "execution V2 disposition differs from its Population V4/admission row".to_owned(),
        );
    }
    row.admission
        .require_matches_verdict(*decision.verdict())
        .map_err(|why| format!("execution V2 admission summary mismatch: {why}"))
}

fn validate_parameter_for_row(
    population_v4: &CompletionReceiptV4,
    population_v4_digest: [u8; 32],
    parameters: &ExecutionParametersV1,
    row: &PopulationRowV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let v2 = population_v4.v3().v2();
    let expected_grid = match row.direction {
        TradeDirectionV1::Long => v2.identities.exit_grids.long,
        TradeDirectionV1::Short => v2.identities.exit_grids.short,
    };
    if parameters.population_id() != row.population_id
        || parameters.population_v4_digest() != population_v4_digest
        || parameters.direction() != row.direction
        || parameters.instrument_family() != row.instrument_family
        || parameters.rung_seconds() != row.rung_seconds
        || parameters.policy().digest() != expected_grid.policy_digest
        || parameters.expected_resolved_digest() != expected_grid.resolved_digest
    {
        return Err(
            "execution parameters differ from their Population V4 row/side authority".to_owned(),
        );
    }
    Ok(())
}

fn validate_parameter_pair_for_population(
    population_v4: &CompletionReceiptV4,
    population_v4_digest: [u8; 32],
    long: &ExecutionParametersV1,
    short: &ExecutionParametersV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let v2 = population_v4.v3().v2();
    for (expected_direction, parameters, expected_grid) in [
        (TradeDirectionV1::Long, long, v2.identities.exit_grids.long),
        (
            TradeDirectionV1::Short,
            short,
            v2.identities.exit_grids.short,
        ),
    ] {
        if parameters.population_id() != population_v4.population_id()
            || parameters.population_v4_digest() != population_v4_digest
            || parameters.direction() != expected_direction
            || parameters.instrument_family() != v2.instrument_family
            || parameters.rung_seconds() != v2.rung_seconds
            || parameters.policy().digest() != expected_grid.policy_digest
            || parameters.expected_resolved_digest() != expected_grid.resolved_digest
        {
            return Err(
                "execution V2 long/short parameter pair differs from Population V4".to_owned(),
            );
        }
    }
    if long.parameter_id() == short.parameter_id() {
        return Err("execution V2 long and short parameters share one identity".to_owned());
    }
    Ok(())
}

fn validate_reopened_parameter_join(
    disposition: &ExecutionRowDispositionV2,
    row: &PopulationRowV1,
    parameters: &ExecutionParametersV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    if disposition.parameter_id != parameters.parameter_id()
        || disposition.resolution_digest != parameters.expected_resolved_digest()
        || disposition.coordinate != chosen_from_population(row.exit)?
    {
        return Err(
            "execution V2 disposition differs from its dynamic parameter/coordinate authority"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_classification_for_row(
    population_v4: &CompletionReceiptV4,
    parameters: &ExecutionParametersV1,
    row: &PopulationRowV1,
    classification: &ExecutionDispositionV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let expected_side = match row.direction {
        TradeDirectionV1::Long => Side::Long,
        TradeDirectionV1::Short => Side::Short,
    };
    if classification.coordinate() != chosen_from_population(row.exit)? {
        return Err(
            "runner execution coordinate differs from its Population V4 row coordinate".to_owned(),
        );
    }
    if classification.resolution_digest() != parameters.expected_resolved_digest()
        || classification.mask().words() != row.mask_words
        || classification.horizon() != parameters.horizon()
        || classification.evaluation_spec_fingerprint() != *parameters.evaluation_fingerprint()
        || classification.side() != expected_side
    {
        return Err(
            "runner execution classification differs from its parameter/row context".to_owned(),
        );
    }
    let v2 = population_v4.v3().v2();
    let expected_grid = match row.direction {
        TradeDirectionV1::Long => v2.identities.exit_grids.long,
        TradeDirectionV1::Short => v2.identities.exit_grids.short,
    };
    let rebuilt_strategy = derive_strategy_digest_from_disposition_v1(
        row.population_id,
        row.instrument_family,
        row.rung_seconds,
        v2.identities.evaluation_policy_digest,
        row.direction,
        expected_grid.policy_digest,
        expected_grid.resolved_digest,
        row.exit,
        classification,
    )?;
    if rebuilt_strategy != row.strategy_digest {
        return Err(
            "runner execution classification belongs to another canonical strategy/run".to_owned(),
        );
    }
    for (name, digest) in [
        (
            "runner execution resolution",
            classification.resolution_digest(),
        ),
        ("runner execution run", classification.run_id().bytes()),
        ("runner execution column", classification.column_digest()),
        ("runner execution context", classification.context_digest()),
        (
            "runner execution terminal disposition",
            classification.disposition_digest(),
        ),
    ] {
        require_digest(name, &digest)?;
    }
    Ok(())
}

fn chosen_from_population(
    coordinate: crate::population::ExitCoordinateV1,
) -> Result<Chosen, ExecutionDispositionRefusalV2> {
    let to_index = |value: Option<u32>, axis: &str| {
        value
            .map(|value| {
                usize::try_from(value)
                    .map_err(|_| format!("Population V4 {axis} coordinate does not fit usize"))
            })
            .transpose()
    };
    Ok(Chosen {
        stop: to_index(coordinate.stop, "stop")?,
        target: to_index(coordinate.target, "target")?,
        tsl: to_index(coordinate.tsl, "TSL")?,
        ttp: coordinate
            .ttp
            .map(|(arm, trail)| {
                Ok::<Ttp, ExecutionDispositionRefusalV2>(Ttp {
                    arm: usize::try_from(arm).map_err(|_| {
                        "Population V4 TTP arm coordinate does not fit usize".to_owned()
                    })?,
                    trail: usize::try_from(trail).map_err(|_| {
                        "Population V4 TTP trail coordinate does not fit usize".to_owned()
                    })?,
                })
            })
            .transpose()?,
    })
}

fn chosen_slots(coordinate: Chosen) -> Result<[u64; 5], ExecutionDispositionRefusalV2> {
    let index = |value: Option<usize>, axis: &str| {
        value.map_or(Ok(u64::MAX), |value| {
            let encoded = u64::try_from(value)
                .map_err(|_| format!("execution {axis} coordinate does not fit u64"))?;
            if encoded == u64::MAX {
                return Err(format!(
                    "execution {axis} coordinate u64::MAX is reserved for None"
                ));
            }
            Ok(encoded)
        })
    };
    let (arm, trail) = match coordinate.ttp {
        Some(ttp) => (
            index(Some(ttp.arm), "TTP arm")?,
            index(Some(ttp.trail), "TTP trail")?,
        ),
        None => (u64::MAX, u64::MAX),
    };
    Ok([
        index(coordinate.stop, "stop")?,
        index(coordinate.target, "target")?,
        index(coordinate.tsl, "TSL")?,
        arm,
        trail,
    ])
}

fn validate_chosen(coordinate: Chosen) -> Result<(), ExecutionDispositionRefusalV2> {
    let _ = chosen_slots(coordinate)?;
    Ok(())
}

fn hash_chosen(
    hasher: &mut brutex_core::blake3::Hasher,
    coordinate: Chosen,
) -> Result<(), ExecutionDispositionRefusalV2> {
    for slot in chosen_slots(coordinate)? {
        hasher.update(&slot.to_le_bytes());
    }
    Ok(())
}

fn encode_chosen(
    encoder: &mut Encoder<'_>,
    coordinate: Chosen,
) -> Result<(), ExecutionDispositionRefusalV2> {
    for slot in chosen_slots(coordinate)? {
        encoder.u64(slot)?;
    }
    Ok(())
}

fn decode_chosen(decoder: &mut Decoder<'_>) -> Result<Chosen, ExecutionDispositionRefusalV2> {
    let slots = [
        decoder.u64()?,
        decoder.u64()?,
        decoder.u64()?,
        decoder.u64()?,
        decoder.u64()?,
    ];
    let decode = |value: u64, axis: &str| {
        if value == u64::MAX {
            Ok(None)
        } else {
            usize::try_from(value)
                .map(Some)
                .map_err(|_| format!("execution {axis} coordinate does not fit usize"))
        }
    };
    let arm = decode(slots[3], "TTP arm")?;
    let trail = decode(slots[4], "TTP trail")?;
    let ttp = match (arm, trail) {
        (None, None) => None,
        (Some(arm), Some(trail)) => Some(Ttp { arm, trail }),
        _ => {
            return Err(
                "execution TTP coordinate has only one of its arm/trail indexes".to_owned(),
            );
        }
    };
    Ok(Chosen {
        stop: decode(slots[0], "stop")?,
        target: decode(slots[1], "target")?,
        tsl: decode(slots[2], "TSL")?,
        ttp,
    })
}

/// Whether a complete V2 authority was durably appended or exactly reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionDispositionCommitV2 {
    /// The row block was synced and its completion receipt was synced last.
    Written(ExecutionCapabilityCompletionV2),
    /// The existing receipt and every committed row matched byte-for-byte.
    Reused(ExecutionCapabilityCompletionV2),
}

impl ExecutionDispositionCommitV2 {
    /// The written or reused semantic completion authority.
    #[must_use]
    pub const fn completion(self) -> ExecutionCapabilityCompletionV2 {
        match self {
            Self::Written(completion) | Self::Reused(completion) => completion,
        }
    }
}

/// One bounded, owned page of committed execution dispositions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionDispositionPageV2 {
    completion_id: [u8; 32],
    population_id: [u8; 32],
    total_rows: u64,
    offset: u64,
    rows: Vec<ExecutionRowDispositionV2>,
}

impl ExecutionDispositionPageV2 {
    /// Complete execution authority from which this page was read.
    #[must_use]
    pub const fn completion_id(&self) -> [u8; 32] {
        self.completion_id
    }

    /// Population shared by every row in this page.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Total committed row cardinality, not merely this page's length.
    #[must_use]
    pub const fn total_rows(&self) -> u64 {
        self.total_rows
    }

    /// Zero-based row sequence at which this page begins.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Number of rows returned in this bounded page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the requested bounded page contains no row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Read-only iteration in exact Population V4 row order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ExecutionRowDispositionV2> {
        self.rows.iter()
    }

    /// Owned page rows for an API/CLI boundary that must transfer them.
    #[must_use]
    pub fn into_rows(self) -> Vec<ExecutionRowDispositionV2> {
        self.rows
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformGeneration,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration;

#[derive(Clone, Debug)]
struct CommittedExecutionAuthorityV2 {
    receipt: ExecutionCapabilityCompletionV2,
    parameters: [ExecutionParametersV1; 2],
}

type ExecutionCompletionIndexV2 = HashMap<[u8; 32], CommittedExecutionAuthorityV2>;
type ExecutionRowIndexV2 = HashMap<([u8; 32], u64), ExecutionRowDispositionV2>;
type ExecutionIndexesV2 = (ExecutionCompletionIndexV2, ExecutionRowIndexV2);

/// Append-only four-file execution-disposition authority.
///
/// The row file can contain sealed crash orphans. Only a completion receipt
/// commits a contiguous row block. Every operation rechecks all four open file
/// generations under the common population writer lock, so a stale handle
/// cannot serve a newly appended or same-length-replaced authority.
#[derive(Debug)]
pub struct ExecutionDispositionLedgerV2 {
    parameter_file: File,
    percentile_file: File,
    row_file: File,
    completion_file: File,
    writer_lock: File,
    parameter_path: PathBuf,
    percentile_path: PathBuf,
    row_path: PathBuf,
    completion_path: PathBuf,
    parameter_generation: FileGeneration,
    percentile_generation: FileGeneration,
    row_generation: FileGeneration,
    completion_generation: FileGeneration,
    completions: ExecutionCompletionIndexV2,
    rows: ExecutionRowIndexV2,
    writable: bool,
}

impl ExecutionDispositionLedgerV2 {
    /// Side-specific dynamic execution-parameter scalar records.
    #[must_use]
    pub fn parameter_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("execution-disposition-parameters-v2.bin")
    }

    /// Runtime-sized percentile atoms needed to reconstruct V2 parameters.
    #[must_use]
    pub fn percentile_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("execution-disposition-percentiles-v2.bin")
    }

    /// Fixed-stride row dispositions, including policy refusals.
    #[must_use]
    pub fn row_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("execution-row-dispositions-v2.bin")
    }

    /// Receipt-last complete execution authorities.
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("execution-capability-completions-v2.bin")
    }

    fn lock_path(root: &Path) -> PathBuf {
        root.join("results").join("population-write.lock")
    }

    /// Creates missing V2 files and fully validates every sealed record and
    /// committed block before returning a writable handle.
    ///
    /// # Errors
    ///
    /// Refuses I/O/lock/header/version/stride/reserve/seal failures, ragged
    /// tails, unknown tags or refusal bits, invalid/overlapping completion
    /// blocks, duplicate populations, or any count/digest/order mismatch.
    pub fn open(root: &Path) -> Result<Self, ExecutionDispositionRefusalV2> {
        let results = root.join("results");
        ensure_results_directory(root, &results)?;
        let lock_path = Self::lock_path(root);
        let writer_lock = open_or_create(&lock_path)?;
        writer_lock.lock().map_err(|why| {
            format!(
                "{} could not be exclusively locked: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let parameter_path = Self::parameter_path(root);
            let percentile_path = Self::percentile_path(root);
            let row_path = Self::row_path(root);
            let completion_path = Self::completion_path(root);
            let mut parameter_file = open_or_create(&parameter_path)?;
            let mut percentile_file = open_or_create(&percentile_path)?;
            let mut row_file = open_or_create(&row_path)?;
            let mut completion_file = open_or_create(&completion_path)?;
            ensure_record_header(
                &mut parameter_file,
                &parameter_path,
                PARAMETER_MAGIC,
                EXECUTION_PARAMETER_STRIDE,
            )?;
            ensure_record_header(
                &mut percentile_file,
                &percentile_path,
                PERCENTILE_MAGIC,
                EXECUTION_PERCENTILE_STRIDE,
            )?;
            ensure_record_header(
                &mut row_file,
                &row_path,
                ROW_MAGIC,
                EXECUTION_DISPOSITION_ROW_STRIDE_V2,
            )?;
            ensure_record_header(
                &mut completion_file,
                &completion_path,
                COMPLETION_MAGIC,
                EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2,
            )?;
            sync_directory(&results)?;
            Self::from_files(
                parameter_file,
                percentile_file,
                row_file,
                completion_file,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                parameter_path,
                percentile_path,
                row_path,
                completion_path,
                true,
            )
        })();
        release_initial_lock(&writer_lock, &lock_path, opened)
    }

    /// Opens existing V2 files read-only and rebuilds all exact indexes.
    ///
    /// Opening scans all durable rows and receipts and is O(stored records).
    ///
    /// # Errors
    ///
    /// Has the same fail-closed validation surface as [`Self::open`] and also
    /// refuses any missing file.
    pub fn open_read(root: &Path) -> Result<Self, ExecutionDispositionRefusalV2> {
        let lock_path = Self::lock_path(root);
        let writer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", lock_path.display()))?;
        let opened = (|| {
            let parameter_path = Self::parameter_path(root);
            let percentile_path = Self::percentile_path(root);
            let row_path = Self::row_path(root);
            let completion_path = Self::completion_path(root);
            let parameter_file = File::open(&parameter_path).map_err(|why| {
                format!("{} could not be opened: {why}", parameter_path.display())
            })?;
            let percentile_file = File::open(&percentile_path).map_err(|why| {
                format!("{} could not be opened: {why}", percentile_path.display())
            })?;
            let row_file = File::open(&row_path)
                .map_err(|why| format!("{} could not be opened: {why}", row_path.display()))?;
            let completion_file = File::open(&completion_path).map_err(|why| {
                format!("{} could not be opened: {why}", completion_path.display())
            })?;
            Self::from_files(
                parameter_file,
                percentile_file,
                row_file,
                completion_file,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                parameter_path,
                percentile_path,
                row_path,
                completion_path,
                false,
            )
        })();
        release_initial_lock(&writer_lock, &lock_path, opened)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the four independently generation-checked files and their four explicit paths are the authority boundary"
    )]
    fn from_files(
        mut parameter_file: File,
        mut percentile_file: File,
        mut row_file: File,
        mut completion_file: File,
        writer_lock: File,
        parameter_path: PathBuf,
        percentile_path: PathBuf,
        row_path: PathBuf,
        completion_path: PathBuf,
        writable: bool,
    ) -> Result<Self, ExecutionDispositionRefusalV2> {
        check_record_file(
            &mut parameter_file,
            &parameter_path,
            PARAMETER_MAGIC,
            EXECUTION_PARAMETER_STRIDE,
        )?;
        check_record_file(
            &mut percentile_file,
            &percentile_path,
            PERCENTILE_MAGIC,
            EXECUTION_PERCENTILE_STRIDE,
        )?;
        check_record_file(
            &mut row_file,
            &row_path,
            ROW_MAGIC,
            EXECUTION_DISPOSITION_ROW_STRIDE_V2,
        )?;
        check_record_file(
            &mut completion_file,
            &completion_path,
            COMPLETION_MAGIC,
            EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2,
        )?;
        let durable_parameters = scan_records::<EXECUTION_PARAMETER_STRIDE, ParameterScalarV1>(
            &mut parameter_file,
            &parameter_path,
            ParameterScalarV1::from_bytes,
        )?;
        let durable_percentiles = scan_records::<EXECUTION_PERCENTILE_STRIDE, PercentileRecordV1>(
            &mut percentile_file,
            &percentile_path,
            PercentileRecordV1::from_bytes,
        )?;
        let durable_rows =
            scan_records::<EXECUTION_DISPOSITION_ROW_STRIDE_V2, ExecutionRowDispositionV2>(
                &mut row_file,
                &row_path,
                ExecutionRowDispositionV2::from_bytes,
            )?;
        let durable_completions = scan_records::<
            EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2,
            ExecutionCapabilityCompletionV2,
        >(
            &mut completion_file,
            &completion_path,
            ExecutionCapabilityCompletionV2::from_bytes,
        )?;
        let (completions, rows) = build_disposition_indexes(
            &durable_parameters,
            &durable_percentiles,
            &durable_rows,
            &durable_completions,
        )?;
        let parameter_generation = file_generation(&parameter_file, &parameter_path)?;
        let percentile_generation = file_generation(&percentile_file, &percentile_path)?;
        let row_generation = file_generation(&row_file, &row_path)?;
        let completion_generation = file_generation(&completion_file, &completion_path)?;
        Ok(Self {
            parameter_file,
            percentile_file,
            row_file,
            completion_file,
            writer_lock,
            parameter_path,
            percentile_path,
            row_path,
            completion_path,
            parameter_generation,
            percentile_generation,
            row_generation,
            completion_generation,
            completions,
            rows,
            writable,
        })
    }

    /// Average-O(1) complete-authority lookup after two generation checks.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses lock failure or any append, replacement or same-length mutation
    /// since this handle was opened.
    pub fn completion(
        &mut self,
        population_id: [u8; 32],
    ) -> Result<Option<ExecutionCapabilityCompletionV2>, ExecutionDispositionRefusalV2> {
        self.with_shared_lock("an execution V2 completion lookup", |ledger| {
            ledger.require_generations_unchanged()?;
            Ok(ledger
                .completions
                .get(&population_id)
                .map(|authority| authority.receipt))
        })
    }

    /// Reconstructed dynamic parameter authority for one exact side.
    ///
    /// # Errors
    ///
    /// Refuses lock failure or any post-open change to any of the four files.
    pub fn parameters(
        &mut self,
        population_id: [u8; 32],
        direction: TradeDirectionV1,
    ) -> Result<Option<ExecutionParametersV1>, ExecutionDispositionRefusalV2> {
        self.with_shared_lock("an execution V2 parameter lookup", |ledger| {
            ledger.require_generations_unchanged()?;
            Ok(ledger
                .completions
                .get(&population_id)
                .map(|authority| match direction {
                    TradeDirectionV1::Long => authority.parameters[0].clone(),
                    TradeDirectionV1::Short => authority.parameters[1].clone(),
                }))
        })
    }

    /// Average-O(1) lookup of one exact committed Population V4 row.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses lock failure or any post-open file generation change.
    pub fn row(
        &mut self,
        population_id: [u8; 32],
        row_sequence: u64,
    ) -> Result<Option<ExecutionRowDispositionV2>, ExecutionDispositionRefusalV2> {
        self.with_shared_lock("an execution V2 row lookup", |ledger| {
            ledger.require_generations_unchanged()?;
            Ok(ledger.rows.get(&(population_id, row_sequence)).copied())
        })
    }

    /// Returns at most 256 committed rows in exact sequence order.
    ///
    /// Zero `limit` is valid. A missing population returns `None`; an offset at
    /// or beyond the end returns an empty page carrying the complete authority.
    ///
    /// # Errors
    ///
    /// Refuses an over-ceiling limit, stale files, arithmetic/allocation
    /// failure, or a missing indexed row inside a committed completion.
    pub fn page(
        &mut self,
        population_id: [u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<Option<ExecutionDispositionPageV2>, ExecutionDispositionRefusalV2> {
        if limit > MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2 {
            return Err(format!(
                "execution disposition page limit {limit} exceeds the fixed {MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2}-row ceiling"
            ));
        }
        self.with_shared_lock("an execution V2 bounded page", |ledger| {
            ledger.require_generations_unchanged()?;
            let Some(authority) = ledger.completions.get(&population_id) else {
                return Ok(None);
            };
            let completion = authority.receipt;
            let count = completion.row_count.saturating_sub(offset).min(limit);
            let capacity = usize::try_from(count)
                .map_err(|_| "bounded execution disposition page does not fit usize".to_owned())?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(capacity).map_err(|why| {
                format!("bounded execution disposition page allocation refused: {why}")
            })?;
            for nth in 0..count {
                let sequence = offset.checked_add(nth).ok_or_else(|| {
                    "execution disposition page sequence overflowed u64".to_owned()
                })?;
                rows.push(
                    ledger
                        .rows
                        .get(&(population_id, sequence))
                        .copied()
                        .ok_or_else(|| {
                            "committed execution completion has a missing indexed row".to_owned()
                        })?,
                );
            }
            Ok(Some(ExecutionDispositionPageV2 {
                completion_id: completion.completion_id,
                population_id,
                total_rows: completion.row_count,
                offset,
                rows,
            }))
        })
    }

    /// Appends all rows, syncs them, then appends and syncs the receipt last.
    ///
    /// A rerun reuses an existing authority only when its semantic receipt and
    /// every durable row are exactly equal. Valid rows left by a crash remain
    /// unreferenced evidence and never become authoritative by proximity.
    ///
    /// # Errors
    ///
    /// Refuses read-only/stale handles, changed reruns, invalid prepared state,
    /// lock/I/O/sync failures, or count/index/range overflow.
    pub fn append_complete(
        &mut self,
        prepared: &PreparedExecutionDispositionsV2,
    ) -> Result<ExecutionDispositionCommitV2, ExecutionDispositionRefusalV2> {
        if !self.writable {
            return Err(
                "a read-only execution-disposition ledger cannot append; reopen with open"
                    .to_owned(),
            );
        }
        self.writer_lock.lock().map_err(|why| {
            format!("the population writer lock could not be exclusively locked: {why}")
        })?;
        let attempted = self.append_complete_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be unlocked: {why}"));
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreparedExecutionDispositionsV2,
    ) -> Result<ExecutionDispositionCommitV2, ExecutionDispositionRefusalV2> {
        self.require_generations_unchanged()?;
        let semantic = ExecutionCapabilityCompletionV2::for_prepared(prepared, 0, 0, 0)?;
        if let Some(existing) = self.completions.get(&prepared.population_id) {
            if existing.receipt.completion_id != semantic.completion_id
                || !existing.receipt.same_semantics(prepared)
                || existing.parameters != prepared.parameters
            {
                return Err(
                    "population already has a different execution-disposition authority".to_owned(),
                );
            }
            self.require_exact_reuse_rows(prepared)?;
            self.sync_all()?;
            return Ok(ExecutionDispositionCommitV2::Reused(existing.receipt));
        }

        let parameter_first = record_count(
            &self.parameter_file,
            &self.parameter_path,
            EXECUTION_PARAMETER_STRIDE,
        )?;
        let percentile_first = record_count(
            &self.percentile_file,
            &self.percentile_path,
            EXECUTION_PERCENTILE_STRIDE,
        )?;
        let row_first = record_count(
            &self.row_file,
            &self.row_path,
            EXECUTION_DISPOSITION_ROW_STRIDE_V2,
        )?;
        let completion = ExecutionCapabilityCompletionV2::for_prepared(
            prepared,
            parameter_first,
            percentile_first,
            row_first,
        )?;
        append_encoded(
            &mut self.parameter_file,
            &self.parameter_path,
            "execution disposition parameters",
            prepared.scalars.iter().map(ParameterScalarV1::to_bytes),
        )?;
        sync_record_file(&self.parameter_file, &self.parameter_path)?;
        append_encoded(
            &mut self.percentile_file,
            &self.percentile_path,
            "execution disposition percentiles",
            prepared
                .percentiles
                .iter()
                .copied()
                .map(PercentileRecordV1::to_bytes),
        )?;
        sync_record_file(&self.percentile_file, &self.percentile_path)?;
        append_encoded(
            &mut self.row_file,
            &self.row_path,
            "execution disposition rows",
            prepared
                .rows
                .iter()
                .copied()
                .map(ExecutionRowDispositionV2::to_bytes),
        )?;
        sync_record_file(&self.row_file, &self.row_path)?;
        append_encoded(
            &mut self.completion_file,
            &self.completion_path,
            "execution disposition completion receipt",
            [completion.to_bytes()],
        )?;
        sync_record_file(&self.completion_file, &self.completion_path)?;
        self.refresh_generations()?;
        self.index_prepared(prepared, &completion)?;
        Ok(ExecutionDispositionCommitV2::Written(completion))
    }

    fn require_exact_reuse_rows(
        &self,
        prepared: &PreparedExecutionDispositionsV2,
    ) -> Result<(), ExecutionDispositionRefusalV2> {
        for row in &prepared.rows {
            if self.rows.get(&(prepared.population_id, row.row_sequence)) != Some(row) {
                return Err(
                    "existing execution-disposition authority does not exactly reuse every row"
                        .to_owned(),
                );
            }
        }
        Ok(())
    }

    fn index_prepared(
        &mut self,
        prepared: &PreparedExecutionDispositionsV2,
        completion: &ExecutionCapabilityCompletionV2,
    ) -> Result<(), ExecutionDispositionRefusalV2> {
        self.rows
            .try_reserve(prepared.rows.len())
            .map_err(|why| format!("execution disposition row index allocation refused: {why}"))?;
        for row in prepared.rows.iter().copied() {
            if self
                .rows
                .insert((prepared.population_id, row.row_sequence), row)
                .is_some()
            {
                return Err("duplicate committed execution disposition row".to_owned());
            }
        }
        if self
            .completions
            .insert(
                prepared.population_id,
                CommittedExecutionAuthorityV2 {
                    receipt: *completion,
                    parameters: prepared.parameters.clone(),
                },
            )
            .is_some()
        {
            return Err("duplicate execution disposition completion".to_owned());
        }
        Ok(())
    }

    fn with_shared_lock<T>(
        &mut self,
        purpose: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, ExecutionDispositionRefusalV2>,
    ) -> Result<T, ExecutionDispositionRefusalV2> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!("the population writer lock could not be shared-locked for {purpose}: {why}")
        })?;
        let attempted = operation(self);
        let released = self.writer_lock.unlock().map_err(|why| {
            format!("the population writer lock could not be unlocked after {purpose}: {why}")
        });
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn require_generations_unchanged(&self) -> Result<(), ExecutionDispositionRefusalV2> {
        for (expected, file, path, subject) in [
            (
                self.parameter_generation,
                &self.parameter_file,
                &self.parameter_path,
                "execution disposition parameter file",
            ),
            (
                self.percentile_generation,
                &self.percentile_file,
                &self.percentile_path,
                "execution disposition percentile file",
            ),
            (
                self.row_generation,
                &self.row_file,
                &self.row_path,
                "execution disposition row file",
            ),
            (
                self.completion_generation,
                &self.completion_file,
                &self.completion_path,
                "execution disposition completion file",
            ),
        ] {
            if file_generation(file, path)? != expected {
                return Err(format!(
                    "{} {subject} changed after open; reopen and fully revalidate before use",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    fn refresh_generations(&mut self) -> Result<(), ExecutionDispositionRefusalV2> {
        self.parameter_generation = file_generation(&self.parameter_file, &self.parameter_path)?;
        self.percentile_generation = file_generation(&self.percentile_file, &self.percentile_path)?;
        self.row_generation = file_generation(&self.row_file, &self.row_path)?;
        self.completion_generation = file_generation(&self.completion_file, &self.completion_path)?;
        Ok(())
    }

    fn sync_all(&self) -> Result<(), ExecutionDispositionRefusalV2> {
        sync_record_file(&self.parameter_file, &self.parameter_path)?;
        sync_record_file(&self.percentile_file, &self.percentile_path)?;
        sync_record_file(&self.row_file, &self.row_path)?;
        sync_record_file(&self.completion_file, &self.completion_path)
    }
}

/// Admits only an existing configured authority root, then creates at most its
/// single `results` child. Recursive creation is intentionally forbidden: if a
/// removable `/Volumes/...` root has vanished, this function must not recreate
/// that pathname on the internal disk.
///
/// This closes only missing/non-directory recreation. D-0473 remains the
/// explicit limit for pathname replacement between checks and for proving that
/// a later mount is the same physical device.
fn ensure_results_directory(
    root: &Path,
    results: &Path,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let root_metadata = fs::metadata(root).map_err(|why| {
        format!(
            "execution V2 authority root {} must already exist as a directory: {why}",
            root.display()
        )
    })?;
    if !root_metadata.is_dir() {
        return Err(format!(
            "execution V2 authority root {} is not a directory",
            root.display()
        ));
    }

    match fs::create_dir(results) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::metadata(results).map_err(|metadata_why| {
                format!(
                    "execution V2 results directory {} could not be inspected after an existing-path result ({why}): {metadata_why}",
                    results.display()
                )
            })?;
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(format!(
                    "execution V2 results directory {} exists but is not a directory",
                    results.display()
                ))
            }
        }
        Err(why) => Err(format!(
            "execution V2 results directory {} could not be created beneath the admitted root: {why}",
            results.display()
        )),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "reopen deliberately validates all four durable blocks and their cross-record identities in one ordered pass"
)]
fn build_disposition_indexes(
    durable_parameters: &[ParameterScalarV1],
    durable_percentiles: &[PercentileRecordV1],
    durable_rows: &[ExecutionRowDispositionV2],
    durable_completions: &[ExecutionCapabilityCompletionV2],
) -> Result<ExecutionIndexesV2, ExecutionDispositionRefusalV2> {
    let mut completions = HashMap::new();
    completions
        .try_reserve(durable_completions.len())
        .map_err(|why| format!("execution completion index allocation refused: {why}"))?;
    let committed_count = durable_completions
        .iter()
        .try_fold(0_usize, |sum, receipt| {
            let count = usize::try_from(receipt.row_count)
                .map_err(|_| "committed execution row count does not fit usize".to_owned())?;
            sum.checked_add(count)
                .ok_or_else(|| "committed execution row count overflowed usize".to_owned())
        })?;
    let mut rows = HashMap::new();
    rows.try_reserve(committed_count)
        .map_err(|why| format!("execution row index allocation refused: {why}"))?;

    let mut prior_parameter_end = 0_u64;
    let mut prior_percentile_end = 0_u64;
    let mut prior_row_end = 0_u64;
    for completion in durable_completions.iter().copied() {
        completion.validate()?;
        require_monotonic_block(
            completion.parameter_first,
            completion.parameter_count,
            &mut prior_parameter_end,
            "execution parameter",
        )?;
        require_monotonic_block(
            completion.percentile_first,
            completion.percentile_count,
            &mut prior_percentile_end,
            "execution percentile",
        )?;
        require_monotonic_block(
            completion.row_first,
            completion.disposition_count,
            &mut prior_row_end,
            "execution disposition",
        )?;
        let parameter_block = block_slice(
            durable_parameters,
            completion.parameter_first,
            completion.parameter_count,
            "execution parameter",
        )?;
        let percentile_block = block_slice(
            durable_percentiles,
            completion.percentile_first,
            completion.percentile_count,
            "execution percentile",
        )?;
        let row_block = block_slice(
            durable_rows,
            completion.row_first,
            completion.disposition_count,
            "execution disposition",
        )?;
        if parameter_digest_records(parameter_block)? != completion.ordered_parameter_digest
            || percentile_digest_records(percentile_block)? != completion.ordered_percentile_digest
        {
            return Err(
                "execution V2 completion parameter/percentile digest differs from its records"
                    .to_owned(),
            );
        }
        let [long_scalar, short_scalar] = parameter_block else {
            return Err("execution V2 parameter block is not exact long+short records".to_owned());
        };
        if long_scalar.direction() != TradeDirectionV1::Long
            || short_scalar.direction() != TradeDirectionV1::Short
            || long_scalar.parameter_id() != completion.long_parameter_id
            || short_scalar.parameter_id() != completion.short_parameter_id
            || long_scalar.population_id() != completion.population_id
            || short_scalar.population_id() != completion.population_id
        {
            return Err("execution V2 parameter block is reordered, copied or foreign".to_owned());
        }
        let long_count = long_scalar.percentile_count()?;
        let short_count = short_scalar.percentile_count()?;
        if long_count
            .checked_add(short_count)
            .ok_or_else(|| "execution V2 percentile counts overflow u64".to_owned())?
            != completion.percentile_count
        {
            return Err(
                "execution V2 parameter percentile counts differ from the completion".to_owned(),
            );
        }
        let long_end = usize::try_from(long_count)
            .map_err(|_| "execution V2 long percentile count does not fit usize".to_owned())?;
        let (long_percentiles, short_percentiles) = percentile_block
            .split_at_checked(long_end)
            .ok_or_else(|| "execution V2 long percentile block is incomplete".to_owned())?;
        let long = long_scalar.reconstruct(long_percentiles)?;
        let short = short_scalar.reconstruct(short_percentiles)?;
        validate_reopened_parameter_pair(&completion, &long, &short)?;
        validate_committed_disposition_block(&completion, row_block, [&long, &short])?;
        if completions.contains_key(&completion.population_id) {
            return Err("duplicate execution V2 completion for one population".to_owned());
        }
        for row in row_block.iter().copied() {
            if rows
                .insert((completion.population_id, row.row_sequence), row)
                .is_some()
            {
                return Err("duplicate committed execution disposition row".to_owned());
            }
        }
        completions.insert(
            completion.population_id,
            CommittedExecutionAuthorityV2 {
                receipt: completion,
                parameters: [long, short],
            },
        );
    }
    Ok((completions, rows))
}

fn validate_reopened_parameter_pair(
    completion: &ExecutionCapabilityCompletionV2,
    long: &ExecutionParametersV1,
    short: &ExecutionParametersV1,
) -> Result<(), ExecutionDispositionRefusalV2> {
    if long.parameter_id() != completion.long_parameter_id
        || short.parameter_id() != completion.short_parameter_id
        || long.population_id() != completion.population_id
        || short.population_id() != completion.population_id
        || long.population_v4_digest() != completion.population_v4_digest
        || short.population_v4_digest() != completion.population_v4_digest
        || long.direction() != TradeDirectionV1::Long
        || short.direction() != TradeDirectionV1::Short
        || long.instrument_family() != short.instrument_family()
        || long.rung_seconds() != short.rung_seconds()
    {
        return Err(
            "execution V2 reconstructed parameter pair differs from its completion".to_owned(),
        );
    }
    Ok(())
}

fn validate_committed_disposition_block(
    completion: &ExecutionCapabilityCompletionV2,
    block: &[ExecutionRowDispositionV2],
    parameters: [&ExecutionParametersV1; 2],
) -> Result<(), ExecutionDispositionRefusalV2> {
    if usize_u64(block.len(), "execution disposition block")? != completion.row_count {
        return Err("execution completion row_count differs from its row block".to_owned());
    }
    let mut matrix = AdmissionExecutionMatrixV2::ZERO;
    let mut authorized = 0_u64;
    let mut refused = 0_u64;
    let mut ordered = brutex_core::blake3::Hasher::new();
    ordered.update(ORDERED_ROW_DOMAIN);
    ordered.update(&completion.row_count.to_le_bytes());
    let mut identities = HashSet::new();
    identities
        .try_reserve(block.len())
        .map_err(|why| format!("execution block identity allocation refused: {why}"))?;
    for (nth, row) in block.iter().copied().enumerate() {
        row.validate()?;
        let expected = usize_u64(nth, "execution disposition sequence")?;
        if row.population_id != completion.population_id
            || row.population_v4_digest != completion.population_v4_digest
            || row.admission_completion_digest != completion.admission_completion_digest
            || row.row_sequence != expected
        {
            return Err(
                "execution disposition block is missing, reordered, copied or foreign".to_owned(),
            );
        }
        let expected_parameters = if row.parameter_id == parameters[0].parameter_id() {
            parameters[0]
        } else if row.parameter_id == parameters[1].parameter_id() {
            parameters[1]
        } else {
            return Err(
                "execution disposition row names neither committed side parameter".to_owned(),
            );
        };
        if row.resolution_digest != expected_parameters.expected_resolved_digest() {
            return Err(
                "execution disposition row resolution differs from its committed parameter"
                    .to_owned(),
            );
        }
        if !identities.insert(row.disposition_id) {
            return Err("execution disposition block repeats a row identity".to_owned());
        }
        matrix.push(row.admission_status, row.tag)?;
        match row.tag {
            ExecutionDispositionTagV2::Authorized => {
                authorized = authorized
                    .checked_add(1)
                    .ok_or_else(|| "authorized execution count overflowed u64".to_owned())?;
            }
            ExecutionDispositionTagV2::PolicyRefused => {
                refused = refused
                    .checked_add(1)
                    .ok_or_else(|| "policy-refused execution count overflowed u64".to_owned())?;
            }
        }
        ordered.update(&row.to_bytes()?);
    }
    if ordered.finalize() != completion.ordered_disposition_digest
        || matrix != completion.matrix
        || authorized != completion.authorized_capability_count
        || refused != completion.policy_refused_count
    {
        return Err(
            "execution completion counts, matrix or ordered digest differ from its row block"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_digest(subject: &str, digest: &[u8; 32]) -> Result<(), ExecutionDispositionRefusalV2> {
    if *digest == [0; 32] {
        Err(format!("{subject} is the all-zero non-identity"))
    } else {
        Ok(())
    }
}

fn checked_sum(values: &[u64], subject: &str) -> Result<u64, ExecutionDispositionRefusalV2> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{subject} total overflowed u64"))
    })
}

fn usize_u64(value: usize, subject: &str) -> Result<u64, ExecutionDispositionRefusalV2> {
    u64::try_from(value).map_err(|_| format!("{subject} count does not fit u64"))
}

fn require_monotonic_block(
    first: u64,
    count: u64,
    prior_end: &mut u64,
    subject: &str,
) -> Result<(), ExecutionDispositionRefusalV2> {
    if first < *prior_end {
        return Err(format!(
            "{subject} completion blocks overlap or run backwards"
        ));
    }
    *prior_end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} completion range overflowed u64"))?;
    Ok(())
}

fn block_slice<'a, T>(
    records: &'a [T],
    first: u64,
    count: u64,
    subject: &str,
) -> Result<&'a [T], ExecutionDispositionRefusalV2> {
    let first =
        usize::try_from(first).map_err(|_| format!("{subject} first does not fit usize"))?;
    let count =
        usize::try_from(count).map_err(|_| format!("{subject} count does not fit usize"))?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} block address overflowed usize"))?;
    records
        .get(first..end)
        .ok_or_else(|| format!("{subject} completion points outside its record file"))
}

fn open_or_create(path: &Path) -> Result<File, ExecutionDispositionRefusalV2> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_record_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), ExecutionDispositionRefusalV2> {
    if measured_len(file, path)? != 0 {
        return Ok(());
    }
    let mut raw = [0; HEADER_BYTES_USIZE];
    let mut encoder = Encoder::new(&mut raw);
    encoder.bytes(&magic)?;
    encoder.u32(FORMAT_VERSION_V2)?;
    encoder.u32(
        u32::try_from(HEADER_BYTES_USIZE)
            .map_err(|_| "execution V2 header width does not fit u32".to_owned())?,
    )?;
    encoder.u32(
        u32::try_from(stride)
            .map_err(|_| "execution V2 record stride does not fit u32".to_owned())?,
    )?;
    encoder.zeros(4)?;
    encoder.finish()?;
    file.write_all(&raw)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable V2 header: {why}",
                path.display()
            )
        })
}

fn check_record_file(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let len = measured_len(file, path)?;
    if len < HEADER_BYTES {
        return Err(format!(
            "{} is shorter than its {HEADER_BYTES}-byte V2 header",
            path.display()
        ));
    }
    let mut raw = [0; HEADER_BYTES_USIZE];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    let expected_header = u32::try_from(HEADER_BYTES_USIZE)
        .map_err(|_| "execution V2 header width does not fit u32".to_owned())?;
    let expected_stride = u32::try_from(stride)
        .map_err(|_| "execution V2 record stride does not fit u32".to_owned())?;
    let mut decoder = Decoder::new(&raw);
    if decoder.bytes::<8>()? != magic {
        return Err(format!(
            "{} has the wrong execution-disposition V2 magic",
            path.display()
        ));
    }
    if decoder.u32()? != FORMAT_VERSION_V2 {
        return Err(format!(
            "{} has an unsupported execution-disposition version",
            path.display()
        ));
    }
    if decoder.u32()? != expected_header {
        return Err(format!(
            "{} has a different V2 header width",
            path.display()
        ));
    }
    if decoder.u32()? != expected_stride {
        return Err(format!(
            "{} has a different execution-disposition record stride",
            path.display()
        ));
    }
    decoder.zeros(4, "execution-disposition header reserve")?;
    decoder.finish()?;
    let stride = u64::try_from(stride)
        .map_err(|_| "execution V2 record stride does not fit u64".to_owned())?;
    if !len.saturating_sub(HEADER_BYTES).is_multiple_of(stride) {
        return Err(format!(
            "{} has a ragged/torn tail; no bytes were ignored or padded",
            path.display()
        ));
    }
    Ok(())
}

fn record_count(
    file: &File,
    path: &Path,
    stride: usize,
) -> Result<u64, ExecutionDispositionRefusalV2> {
    let body = measured_len(file, path)?
        .checked_sub(HEADER_BYTES)
        .ok_or_else(|| format!("{} ended before its header", path.display()))?;
    let stride = u64::try_from(stride)
        .map_err(|_| "execution V2 record stride does not fit u64".to_owned())?;
    if !body.is_multiple_of(stride) {
        return Err(format!("{} has a ragged fixed-stride tail", path.display()));
    }
    Ok(body / stride)
}

fn scan_records<const STRIDE: usize, T>(
    file: &mut File,
    path: &Path,
    decode: fn(&[u8; STRIDE]) -> Result<T, ExecutionDispositionRefusalV2>,
) -> Result<Vec<T>, ExecutionDispositionRefusalV2> {
    let count = record_count(file, path, STRIDE)?;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("{} record count does not fit usize", path.display()))?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("{} index allocation refused: {why}", path.display()))?;
    file.seek(SeekFrom::Start(HEADER_BYTES))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    for sequence in 0..count {
        let mut raw = [0; STRIDE];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} record {sequence} could not be read completely: {why}",
                path.display()
            )
        })?;
        records
            .push(decode(&raw).map_err(|why| {
                format!("{} record {sequence} is invalid: {why}", path.display())
            })?);
    }
    Ok(records)
}

fn append_encoded<const STRIDE: usize>(
    file: &mut File,
    path: &Path,
    subject: &str,
    records: impl IntoIterator<Item = Result<[u8; STRIDE], ExecutionDispositionRefusalV2>>,
) -> Result<(), ExecutionDispositionRefusalV2> {
    let at = file
        .seek(SeekFrom::End(0))
        .map_err(|why| format!("{} could not be extended: {why}", path.display()))?;
    for record in records {
        let raw = match record {
            Ok(raw) => raw,
            Err(why) => return Err(rollback_text(file, at, subject, &why)),
        };
        if let Err(why) = file.write_all(&raw) {
            return Err(rollback_io(file, at, subject, &why));
        }
    }
    Ok(())
}

fn sync_record_file(file: &File, path: &Path) -> Result<(), ExecutionDispositionRefusalV2> {
    file.sync_all()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))
}

fn sync_directory(path: &Path) -> Result<(), ExecutionDispositionRefusalV2> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|why| {
            format!(
                "{} could not durably confirm V2 file names: {why}",
                path.display()
            )
        })
}

fn release_initial_lock<T>(
    writer_lock: &File,
    lock_path: &Path,
    result: Result<T, ExecutionDispositionRefusalV2>,
) -> Result<T, ExecutionDispositionRefusalV2> {
    let released = writer_lock
        .unlock()
        .map_err(|why| format!("{} could not be unlocked: {why}", lock_path.display()));
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn measured_len(file: &File, path: &Path) -> Result<u64, ExecutionDispositionRefusalV2> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))
}

fn file_generation(
    file: &File,
    path: &Path,
) -> Result<FileGeneration, ExecutionDispositionRefusalV2> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionDispositionRefusalV2> {
    let of = |metadata: &fs::Metadata| FileGeneration {
        len: metadata.len(),
        platform: PlatformGeneration {
            device: metadata.dev(),
            inode: metadata.ino(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        },
    };
    let held = of(held);
    let named = of(named);
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened file",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionDispositionRefusalV2> {
    let of = |metadata: &fs::Metadata| -> Result<FileGeneration, ExecutionDispositionRefusalV2> {
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial: metadata
                    .volume_serial_number()
                    .ok_or_else(|| format!("{} has no Windows volume serial", path.display()))?,
                file_index: metadata
                    .file_index()
                    .ok_or_else(|| format!("{} has no Windows file index", path.display()))?,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    };
    let held = of(held)?;
    let named = of(named)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened file",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionDispositionRefusalV2> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while it was measured",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

fn rollback_io(file: &File, at: u64, subject: &str, why: &std::io::Error) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be written: {why}; partial bytes were rolled back to {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be written: {why}; rollback to {at} also failed: {and}; reopen refuses the ragged tail"
        ),
    }
}

fn rollback_text(file: &File, at: u64, subject: &str, why: &str) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be encoded: {why}; partial bytes were rolled back to {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be encoded: {why}; rollback to {at} also failed: {and}; reopen refuses the ragged tail"
        ),
    }
}

fn with_seal<const PAYLOAD: usize, const STRIDE: usize>(
    payload: [u8; PAYLOAD],
) -> Result<[u8; STRIDE], ExecutionDispositionRefusalV2> {
    if STRIDE != PAYLOAD.saturating_add(SEAL_BYTES) {
        return Err("execution disposition payload/stride constants disagree".to_owned());
    }
    let mut raw = [0; STRIDE];
    raw.get_mut(..PAYLOAD)
        .ok_or_else(|| "execution disposition payload slot is absent".to_owned())?
        .copy_from_slice(&payload);
    raw.get_mut(PAYLOAD..)
        .ok_or_else(|| "execution disposition seal slot is absent".to_owned())?
        .copy_from_slice(&brutex_core::blake3::hash(&payload));
    Ok(raw)
}

fn checked_payload<const PAYLOAD: usize, const STRIDE: usize>(
    raw: &[u8; STRIDE],
    subject: &str,
) -> Result<[u8; PAYLOAD], ExecutionDispositionRefusalV2> {
    if STRIDE != PAYLOAD.saturating_add(SEAL_BYTES) {
        return Err(format!("{subject} payload/stride constants disagree"));
    }
    let payload: [u8; PAYLOAD] = raw
        .get(..PAYLOAD)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{subject} payload is absent"))?;
    let stored = raw
        .get(PAYLOAD..)
        .ok_or_else(|| format!("{subject} seal is absent"))?;
    if stored != brutex_core::blake3::hash(&payload) {
        return Err(format!("{subject} failed its BLAKE3 seal"));
    }
    Ok(payload)
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    at: usize,
}

impl<'a> Encoder<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ExecutionDispositionRefusalV2> {
        let end = self
            .at
            .checked_add(value.len())
            .ok_or_else(|| "execution disposition encoder offset overflowed usize".to_owned())?;
        self.bytes
            .get_mut(self.at..end)
            .ok_or_else(|| "execution disposition encoder exceeded its fixed record".to_owned())?
            .copy_from_slice(value);
        self.at = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ExecutionDispositionRefusalV2> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), ExecutionDispositionRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ExecutionDispositionRefusalV2> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), ExecutionDispositionRefusalV2> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "execution disposition encoder reserve overflowed usize".to_owned())?;
        let reserve = self.bytes.get_mut(self.at..end).ok_or_else(|| {
            "execution disposition encoder reserve exceeded its record".to_owned()
        })?;
        reserve.fill(0);
        self.at = end;
        Ok(())
    }

    fn finish(self) -> Result<(), ExecutionDispositionRefusalV2> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "execution disposition encoder wrote {} of {} fixed bytes",
                self.at,
                self.bytes.len()
            ))
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], ExecutionDispositionRefusalV2> {
        let end = self
            .at
            .checked_add(N)
            .ok_or_else(|| "execution disposition decoder offset overflowed usize".to_owned())?;
        let value = self
            .bytes
            .get(self.at..end)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| {
                "execution disposition decoder reached a short fixed record".to_owned()
            })?;
        self.at = end;
        Ok(value)
    }

    fn array_32(&mut self) -> Result<[u8; 32], ExecutionDispositionRefusalV2> {
        self.bytes()
    }

    fn u8(&mut self) -> Result<u8, ExecutionDispositionRefusalV2> {
        Ok(self.bytes::<1>()?[0])
    }

    fn u32(&mut self) -> Result<u32, ExecutionDispositionRefusalV2> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }

    fn u64(&mut self) -> Result<u64, ExecutionDispositionRefusalV2> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), ExecutionDispositionRefusalV2> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| format!("{subject} offset overflowed usize"))?;
        let reserve = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| format!("{subject} is absent"))?;
        if reserve.iter().any(|byte| *byte != 0) {
            return Err(format!("{subject} contains unknown non-zero bytes"));
        }
        self.at = end;
        Ok(())
    }

    fn finish(self) -> Result<(), ExecutionDispositionRefusalV2> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "execution disposition decoder consumed {} of {} fixed bytes",
                self.at,
                self.bytes.len()
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "fixed-record integration/adversarial fixtures must fail loudly, retain complete authorities, and mutate exact byte positions"
)]
pub(crate) mod v2_tests {
    use super::*;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use indicators::column::{Column, EvaluationSpecFingerprintV1};
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};
    use pull::calendar::{DayKind, OPEN_MINUTE, kind_of};
    use pull::session::Day;
    use runner::admission::{
        AdmissionEvidenceV1, AdmissionEvidenceValuesV1, AdmissionPolicyDraftV1, AdmissionPolicyV1,
        AdmissionVerdictV1, CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
    };
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExecutionRunV1, ExecutionSeriesV1, ExitGridPolicyV1,
        ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1, RationalPercentileV1,
        ResolvedExitGridV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
    };
    use runner::identity::{Direction, Params, Run};
    use runner::outcome::Horizon;
    use runner::topn::{RankingPolicyV1, Weights};

    use crate::execution_capability::canonical_execution_parameters_fixture_v1;
    use crate::population::{
        AdmissionStatusV1 as PopulationAdmissionStatusV1, AdmissionV1, ClosureV1,
        CompletionReconciliationV2, ExitCellsPerMaskV2, ExitCoordinateV1, InstrumentFamilyV1,
        LongShortExitGridIdentitiesV2, PopulationCommit, PopulationIdentitiesV2,
        RequestedSpanIdentityV1, SideExitGridIdentityV2, TopMetricsV1,
    };
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};

    const MINUTE_MICROS: i64 = 60_000_000;
    const TEST_FEED: &str = "execution-disposition-v2-test-feed";
    const TEST_COMMIT: &str = "execution-disposition-v2-test-commit";
    const TEST_CALENDAR_POLICY: [u8; 32] = [0xA5; 32];

    fn digest(seed: u8) -> [u8; 32] {
        let mut value = [seed.max(1); 32];
        value[31] = seed.wrapping_add(1).max(1);
        value
    }

    fn bars() -> Vec<Candle> {
        let first = 555_i64
            .checked_mul(MINUTE_MICROS)
            .and_then(|value| value.checked_sub(IST_OFFSET_MICROS))
            .expect("regular NSE open timestamp");
        (0_i64..120)
            .map(|index| Candle {
                ts_micros: first + index * MINUTE_MICROS,
                open: 1_000_000 + index.rem_euclid(7),
                high: 1_000_100 + index,
                low: 999_900 - index,
                close: 1_000_000 + index.rem_euclid(11),
                volume: 10 + index,
                open_interest: OI_NULL,
            })
            .collect()
    }

    fn fingerprint(training: &[Candle]) -> EvaluationSpecFingerprintV1 {
        let mut evaluator = Evaluator::new(
            Widths::pinned().expect("pinned evaluator widths"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        );
        Column::build(training, &mut evaluator)
            .evaluation_spec_token()
            .expect("built column carries its evaluator policy")
            .fingerprint_v1()
    }

    fn percentile(numerator: u32, denominator: u32) -> RationalPercentileV1 {
        RationalPercentileV1::new(numerator, denominator).expect("fixture percentile is canonical")
    }

    fn policy(side: Side) -> ExitGridPolicyV1 {
        policy_with_range(side, RangeResolutionV1::PpmCeiling)
    }

    fn policy_with_range(side: Side, range: RangeResolutionV1) -> ExitGridPolicyV1 {
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            range,
            side,
            RungPlanV1::new(
                vec![
                    percentile(1, 5),
                    percentile(2, 5),
                    percentile(3, 5),
                    percentile(4, 5),
                ],
                vec![
                    percentile(1, 5),
                    percentile(2, 5),
                    percentile(3, 5),
                    percentile(4, 5),
                ],
                vec![
                    percentile(1, 5),
                    percentile(2, 5),
                    percentile(3, 5),
                    percentile(4, 5),
                ],
                4,
            )
            .expect("fixture rung plan"),
            RatioLimitsV1::new(1, 100_000, 64).expect("fixture ratio limits"),
            10_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            1_000,
            1_000,
        )
        .expect("fixture exit-grid policy")
    }

    const fn run_params() -> Params {
        Params {
            min_hits: 20,
            ceiling: 1_000_000,
            pair_budget: 2_000_000,
            policy: 7,
        }
    }

    fn execution_horizon() -> Horizon {
        Horizon::bars(15).expect("nonzero fixture horizon")
    }

    fn parameter_pair(
        population_id: [u8; 32],
        population_v4_digest: [u8; 32],
    ) -> [ExecutionParametersV1; 2] {
        let training = bars();
        let instrument = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("canonical NIFTY key");
        let fingerprint = fingerprint(&training);
        let make = |direction: TradeDirectionV1, side: Side, feed: &'static str| {
            let series =
                ExecutionSeriesV1::new(&instrument, feed, "fixture-commit", digest(240), &training)
                    .expect("attested fixture series");
            let resolved = policy(side)
                .resolve_attested(series)
                .expect("fixture policy resolves");
            canonical_execution_parameters_fixture_v1(
                population_id,
                population_v4_digest,
                direction,
                InstrumentFamilyV1::Nifty,
                300,
                execution_horizon(),
                run_params(),
                fingerprint,
                &resolved,
            )
            .expect("fixture execution parameters")
        };
        [
            make(TradeDirectionV1::Long, Side::Long, "fixture-long"),
            make(TradeDirectionV1::Short, Side::Short, "fixture-short"),
        ]
    }

    fn requested_span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("measured full-month fixture span")
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
                                .saturating_add(bucket_minute.saturating_mul(MINUTE_MICROS))
                                .saturating_sub(IST_OFFSET_MICROS);
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

    fn admission_policy(min_support_hits: u64) -> AdmissionPolicyV1 {
        const PPM: u64 = 1_000_000;
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(min_support_hits),
            min_independent_sessions: Some(1),
            min_trades: Some(1),
            max_mae_paisa: Some(u64::MAX),
            min_worst_reward_risk_ppm: Some(0),
            min_win_rate_ppm: Some(0),
            min_wilson_win_rate_ppm: Some(0),
            min_return_drawdown_ppm: Some(0),
            min_weakest_period_return_paisa: Some(i64::MIN),
            max_pbo_ppm: Some(PPM),
            max_fwer_p_value_ppm: Some(PPM),
            max_spa_p_value_ppm: Some(PPM),
            min_decided_folds: Some(1),
            max_ambiguous_fill_rate_ppm: Some(PPM),
            max_gap_affected_rate_ppm: Some(PPM),
            max_session_concentration_ppm: Some(PPM),
            max_largest_trade_profit_share_ppm: Some(PPM),
            max_drawdown_paisa: Some(u64::MAX),
            max_worst_trade_loss_paisa: Some(u64::MAX),
            max_losing_trade_rate_ppm: Some(PPM),
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
            max_white_reality_p_value_ppm: Some(PPM),
            max_romano_wolf_p_value_ppm: Some(PPM),
            require_white_reality_rejection: Some(false),
            require_romano_wolf_rejection: Some(false),
        })
        .expect("fully explicit fixture admission policy")
    }

    fn admission_evidence(status: AdmissionStatusV1) -> AdmissionEvidenceV1 {
        let pbo_measurement = match status {
            AdmissionStatusV1::Unmeasured => ObservedU64V1::Unmeasured,
            AdmissionStatusV1::Refused => ObservedU64V1::Refused,
            AdmissionStatusV1::Admitted | AdmissionStatusV1::Rejected => {
                panic!("public durable AdmissionEvidenceV1 cannot measure PBO")
            }
        };
        AdmissionEvidenceV1::new(AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(100),
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Measured(50),
            max_mae_paisa: ObservedU64V1::Measured(500),
            worst_reward_risk_ppm: ObservedU64V1::Measured(2_000_000),
            win_rate_ppm: ObservedU64V1::Measured(600_000),
            wilson_win_rate_ppm: ObservedU64V1::Measured(550_000),
            return_drawdown_ppm: ObservedU64V1::Measured(3_000_000),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            pbo_ppm: pbo_measurement,
            fwer_p_value_ppm: ObservedU64V1::Measured(50_000),
            spa_p_value_ppm: ObservedU64V1::Measured(50_000),
            decided_folds: ObservedU64V1::Measured(10),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(100_000),
            gap_affected_rate_ppm: ObservedU64V1::Measured(100_000),
            session_concentration_ppm: ObservedU64V1::Measured(300_000),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(200_000),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(10_000),
            worst_trade_loss_paisa: ObservedU64V1::Measured(2_000),
            losing_trade_rate_ppm: ObservedU64V1::Measured(400_000),
            losing_trades: ObservedU64V1::Measured(20),
            pessimistic_profit_paisa: ObservedI64V1::Measured(1_000),
            winning_trades: ObservedU64V1::Measured(30),
            average_win_paisa: ObservedU64V1::Measured(300),
            average_loss_paisa: ObservedU64V1::Measured(150),
            profit_factor_ppm: ObservedU64V1::Measured(2_000_000),
            consecutive_losing_streak: ObservedU64V1::Measured(3),
            consecutive_winning_streak: ObservedU64V1::Measured(3),
            bootstrap_draws: ObservedU64V1::Measured(1_000),
            bootstrap_strategies: ObservedU64V1::Measured(100),
            bootstrap_periods: ObservedU64V1::Measured(300),
            pbo_contributing_folds: pbo_measurement,
            pbo_unrankable_folds: pbo_measurement,
            profitable_oos_folds: ObservedU64V1::Measured(8),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(1_000),
            white_reality_p_value_ppm: ObservedU64V1::Measured(50_000),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(50_000),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        })
        .expect("complete fixture admission evidence")
    }

    fn project_admission(verdict: AdmissionVerdictV1) -> AdmissionV1 {
        let status = match verdict.status() {
            AdmissionStatusV1::Admitted => PopulationAdmissionStatusV1::Admitted,
            AdmissionStatusV1::Rejected => PopulationAdmissionStatusV1::Rejected,
            AdmissionStatusV1::Unmeasured => PopulationAdmissionStatusV1::Unmeasured,
            AdmissionStatusV1::Refused => PopulationAdmissionStatusV1::Refused,
        };
        AdmissionV1 {
            status,
            reasons: verdict.reasons().bits(),
            failed: verdict.failed().bits(),
            unmeasured: verdict.unmeasured().bits(),
            refused: verdict.refused().bits(),
        }
    }

    const fn top_metrics() -> TopMetricsV1 {
        TopMetricsV1 {
            drawdown: 10_000,
            worst_loss: 2_000,
            losing_rate_ppm: 400_000,
            losing_trades: 20,
            loss_ratio_ppm: Some(500_000),
            pessimistic_profit: 1_000,
            winning_trades: 30,
            win_rate_ppm: 600_000,
            reward_to_risk_ppm: Some(2_000_000),
            average_win: 300,
            average_loss: 150,
            assurance_ppm: 550_000,
        }
    }

    fn integration_instrument(family: InstrumentFamilyV1) -> InstrumentKey {
        let symbol = match family {
            InstrumentFamilyV1::Nifty => "NIFTY",
            InstrumentFamilyV1::BankNifty => "BANKNIFTY",
        };
        InstrumentKey::index(Exchange::Nse, symbol).expect("canonical NSE index fixture")
    }

    fn fixture_population_id(
        population_seed: u8,
        family: InstrumentFamilyV1,
        rung_seconds: u32,
    ) -> [u8; 32] {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex.cli.execution-disposition-v2-test-population\0");
        hasher.update(&[population_seed]);
        hasher.update(&[match family {
            InstrumentFamilyV1::Nifty => 1,
            InstrumentFamilyV1::BankNifty => 2,
        }]);
        hasher.update(&rung_seconds.to_le_bytes());
        hasher.finalize()
    }

    fn integration_column(training: &[Candle], thresholds: Thresholds) -> Column {
        let mut evaluator = Evaluator::new(
            Widths::pinned().expect("pinned evaluator widths"),
            Availability::Absent,
            thresholds,
        );
        Column::build(training, &mut evaluator)
    }

    fn strongest_live_mask(column: &Column) -> ([u64; 6], u64) {
        let mut counts = [0_u64; 384];
        for held in column.bits() {
            for (word_index, word) in held.words().into_iter().enumerate() {
                for bit in 0..64_usize {
                    if word & (1_u64 << bit) != 0 {
                        let index = word_index
                            .checked_mul(64)
                            .and_then(|base| base.checked_add(bit))
                            .expect("six-word fixture bit index");
                        counts[index] = counts[index].saturating_add(1);
                    }
                }
            }
        }
        let (index, hits) = counts
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(_, count)| *count)
            .expect("fixed mask count is nonempty");
        assert!(hits > 0, "fixture column must expose one live condition");
        let mut words = [0_u64; 6];
        words[index / 64] = 1_u64 << (index % 64);
        (words, hits)
    }

    fn resolve_grid<'a>(
        side: Side,
        range: RangeResolutionV1,
        instrument: &'a InstrumentKey,
        training: &'a [Candle],
    ) -> (ExecutionSeriesV1<'a>, ResolvedExitGridV1) {
        let series = ExecutionSeriesV1::new(
            instrument,
            TEST_FEED,
            TEST_COMMIT,
            TEST_CALENDAR_POLICY,
            training,
        )
        .expect("attested integration execution series");
        let resolved = policy_with_range(side, range)
            .resolve_attested(series)
            .expect("integration policy resolves");
        (series, resolved)
    }

    fn classify_side(
        resolved: &ResolvedExitGridV1,
        series: ExecutionSeriesV1<'_>,
        column: &Column,
        training: &[Candle],
        mask_words: [u64; 6],
        direction: TradeDirectionV1,
        parameters: Params,
    ) -> (Vec<ExecutionDispositionV1>, ExecutionRunV1) {
        let mask = runner::replay_mask::from_stored_words(mask_words)
            .expect("fixture mask is canonical and live");
        let run = Run {
            mask,
            direction: match direction {
                TradeDirectionV1::Long => Direction::Long,
                TradeDirectionV1::Short => Direction::Short,
            },
            instrument: series.instrument(),
            timeframe: "1min",
            params: parameters,
            data_digest: runner::identity::data_digest(training),
            commit: TEST_COMMIT,
            feed: TEST_FEED,
        };
        let run =
            ExecutionRunV1::new(&run, training, None).expect("canonical integration execution run");
        let evaluated = resolved
            .evaluate_training_grid_attested(series, column, execution_horizon(), run)
            .expect("complete integration grid evaluates");
        let validated = resolved
            .validate_evaluation(&evaluated)
            .expect("complete integration grid validates once");
        let cell_count =
            usize::try_from(resolved.cell_count()).expect("bounded fixture grid count fits usize");
        let classifications = (0..cell_count)
            .map(|ordinal| {
                let coordinate = validated
                    .cell(ordinal)
                    .map(Chosen::from_cell)
                    .expect("validated ordinal has one canonical cell");
                resolved
                    .classify_coordinate(&validated, coordinate)
                    .expect("canonical coordinate has one terminal")
            })
            .collect();
        (classifications, run)
    }

    fn four_mixed_terminals(
        classified: &[ExecutionDispositionV1],
        refused_first: bool,
        extra_authorized: usize,
    ) -> Vec<ExecutionDispositionV1> {
        let authorized = classified
            .iter()
            .filter(|item| item.is_authorized())
            .take(2_usize.saturating_add(extra_authorized))
            .cloned()
            .collect::<Vec<_>>();
        let refused = classified
            .iter()
            .filter(|item| item.is_policy_refused())
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            authorized.len(),
            2_usize.saturating_add(extra_authorized),
            "fixture needs enough unique authorized cells for its requested Top-N floor"
        );
        assert_eq!(refused.len(), 2, "fixture needs two refused cells");
        let mut selected = if refused_first {
            vec![
                refused[0].clone(),
                authorized[0].clone(),
                refused[1].clone(),
                authorized[1].clone(),
            ]
        } else {
            vec![
                authorized[0].clone(),
                refused[0].clone(),
                authorized[1].clone(),
                refused[1].clone(),
            ]
        };
        selected.extend(authorized.into_iter().skip(2));
        selected
    }

    fn exit_coordinate(disposition: &ExecutionDispositionV1) -> ExitCoordinateV1 {
        let chosen = disposition.coordinate();
        let convert = |value: Option<usize>| {
            value
                .map(u32::try_from)
                .transpose()
                .expect("bounded grid coordinate fits u32")
        };
        ExitCoordinateV1 {
            stop: convert(chosen.stop),
            target: convert(chosen.target),
            tsl: convert(chosen.tsl),
            ttp: chosen.ttp.map(|ttp| {
                (
                    u32::try_from(ttp.arm).expect("bounded TTP arm fits u32"),
                    u32::try_from(ttp.trail).expect("bounded TTP trail fits u32"),
                )
            }),
        }
    }

    fn classification_at(
        classifications: &[ExecutionDispositionV1],
        coordinate: Chosen,
    ) -> ExecutionDispositionV1 {
        classifications
            .iter()
            .find(|classification| classification.coordinate() == coordinate)
            .cloned()
            .expect("fixture grid contains the requested canonical coordinate")
    }

    fn integration_identities(
        training: &[Candle],
        column: &Column,
        long: &ResolvedExitGridV1,
        short: &ResolvedExitGridV1,
        admission: &AdmissionPolicyV1,
        ranking: RankingPolicyV1,
    ) -> PopulationIdentitiesV2 {
        let evaluation_policy_digest = column
            .evaluation_spec_token()
            .map(|token| brutex_core::blake3::hash(token.fingerprint_v1().as_bytes()))
            .expect("integration column carries evaluator identity");
        PopulationIdentitiesV2 {
            run_identity: digest(171),
            data_digest: runner::identity::data_digest(training),
            feed_digest: brutex_core::blake3::hash(TEST_FEED.as_bytes()),
            source_commit_digest: brutex_core::blake3::hash(TEST_COMMIT.as_bytes()),
            vocabulary_digest: digest(172),
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
            admission_policy_digest: admission.digest(),
            ranking_policy_digest: ranking.digest(),
            calendar_policy_digest: TEST_CALENDAR_POLICY,
            daily_reference_policy_digest: digest(174),
        }
    }

    /// Canonical public-path V2 authority used by sibling crate-local tests.
    #[derive(Clone, Debug)]
    pub(crate) struct CanonicalExecutionDispositionV2Fixture {
        pub(crate) receipt: CompletionReceiptV4,
        pub(crate) admission_completion: AdmissionCompletionReceiptV1,
        pub(crate) admission_policy: AdmissionPolicyV1,
        pub(crate) ranking_policy: RankingPolicyV1,
        pub(crate) rows: Vec<PopulationRowV1>,
        pub(crate) decisions: Vec<AdmissionDecisionRecordV1>,
        pub(crate) parameters: [ExecutionParametersV1; 2],
        pub(crate) classifications: Vec<ExecutionDispositionV1>,
        pub(crate) dispositions: Vec<ExecutionRowDispositionV2>,
        pub(crate) prepared: PreparedExecutionDispositionsV2,
        pub(crate) training: Vec<Candle>,
        pub(crate) instrument: InstrumentKey,
        pub(crate) column: Column,
        pub(crate) grids: [ResolvedExitGridV1; 2],
        pub(crate) training_runs: [ExecutionRunV1; 2],
        pub(crate) mask_words: [u64; 6],
    }

    impl CanonicalExecutionDispositionV2Fixture {
        /// Reconstructs the exact attested one-minute training-series authority.
        pub(crate) fn training_series(&self) -> ExecutionSeriesV1<'_> {
            ExecutionSeriesV1::new(
                &self.instrument,
                TEST_FEED,
                TEST_COMMIT,
                TEST_CALENDAR_POLICY,
                &self.training,
            )
            .expect("fixture retains its exact training-series identity")
        }
    }

    /// Builds and commits one exact Population V4 plus admission authority,
    /// then prepares its complete V2 execution-disposition authority through
    /// [`PreparedExecutionDispositionsV2::from_ledgers`].
    pub(crate) fn canonical_execution_disposition_v2_fixture(
        store_root: &Path,
        population_seed: u8,
        instrument_family: InstrumentFamilyV1,
        rung_seconds: u32,
        minimum_authorized_classifications: usize,
        ranking_policy: RankingPolicyV1,
    ) -> CanonicalExecutionDispositionV2Fixture {
        assert!(
            [60, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&rung_seconds),
            "execution-disposition fixture accepts only canonical signal rungs"
        );
        let training = runner::synthetic::sessions(8);
        let instrument = integration_instrument(instrument_family);
        let column = integration_column(&training, Thresholds::CLASSICAL);
        let (mask_words, _) = strongest_live_mask(&column);
        let (long_series, long_grid) = resolve_grid(
            Side::Long,
            RangeResolutionV1::PpmCeiling,
            &instrument,
            &training,
        );
        let (short_series, short_grid) = resolve_grid(
            Side::Short,
            RangeResolutionV1::PpmCeiling,
            &instrument,
            &training,
        );
        let (long_all, long_run) = classify_side(
            &long_grid,
            long_series,
            &column,
            &training,
            mask_words,
            TradeDirectionV1::Long,
            run_params(),
        );
        let (short_all, short_run) = classify_side(
            &short_grid,
            short_series,
            &column,
            &training,
            mask_words,
            TradeDirectionV1::Short,
            run_params(),
        );
        let additional_authorized_per_side = minimum_authorized_classifications
            .saturating_sub(1)
            .saturating_add(1)
            / 2;
        let side_count = 4_usize.saturating_add(additional_authorized_per_side);
        let mut classifications =
            four_mixed_terminals(&long_all, false, additional_authorized_per_side);
        classifications.extend(four_mixed_terminals(
            &short_all,
            true,
            additional_authorized_per_side,
        ));
        let admission_policy = admission_policy(10);
        let identities = integration_identities(
            &training,
            &column,
            &long_grid,
            &short_grid,
            &admission_policy,
            ranking_policy,
        );
        let population_id = fixture_population_id(population_seed, instrument_family, rung_seconds);
        let mandatory_statuses = [
            AdmissionStatusV1::Unmeasured,
            AdmissionStatusV1::Unmeasured,
            AdmissionStatusV1::Refused,
            AdmissionStatusV1::Refused,
        ];
        let mut rows = Vec::with_capacity(classifications.len());
        let mut decisions = Vec::with_capacity(classifications.len());
        for (index, classification) in classifications.iter().enumerate() {
            let sequence = u64::try_from(index).expect("fixture sequence fits u64");
            let direction = if index < side_count {
                TradeDirectionV1::Long
            } else {
                TradeDirectionV1::Short
            };
            let side_index = index % side_count;
            let status = mandatory_statuses
                .get(side_index)
                .copied()
                .unwrap_or(AdmissionStatusV1::Unmeasured);
            let evidence = admission_evidence(status);
            let sealed = admission_policy.evaluate_sealed(&evidence);
            assert_eq!(sealed.verdict().status(), status);
            let exit = exit_coordinate(classification);
            let grid = match direction {
                TradeDirectionV1::Long => identities.exit_grids.long,
                TradeDirectionV1::Short => identities.exit_grids.short,
            };
            let strategy_digest = derive_strategy_digest_from_disposition_v1(
                population_id,
                instrument_family,
                rung_seconds,
                identities.evaluation_policy_digest,
                direction,
                grid.policy_digest,
                grid.resolved_digest,
                exit,
                classification,
            )
            .expect("classification produces canonical strategy identity");
            let row = PopulationRowV1 {
                population_id,
                sequence,
                strategy_digest,
                mask_words: classification.mask().words(),
                direction,
                instrument_family,
                closure: ClosureV1::Closed,
                rung_seconds,
                support_hits: 100,
                exit,
                metrics: top_metrics(),
                admission: project_admission(sealed.verdict()),
            };
            let decision = AdmissionDecisionRecordV1::new(
                population_id,
                sequence,
                strategy_digest,
                row.payload_digest().expect("canonical fixture row"),
                &sealed,
            )
            .expect("canonical admission decision");
            rows.push(row);
            decisions.push(decision);
        }
        let receipt = CompletionReceiptV4::for_rows(
            population_id,
            instrument_family,
            rung_seconds,
            &rows,
            requested_span(),
            CompletionReconciliationV2 {
                sweep_trials: 1,
                frequent_itemsets: 1,
                infrequent_itemsets: 0,
                closed_itemsets: 1,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(
                    u64::try_from(side_count).expect("fixture side count fits u64"),
                    u64::try_from(side_count).expect("fixture side count fits u64"),
                )
                .expect("equal nonzero fixture side populations"),
                extinction_depth: 1,
                extinction_complete: true,
                closure_complete: true,
            },
            identities,
            complete_calendar(rung_seconds),
            complete_calendar(60),
        )
        .expect("canonical Population V4 fixture receipt");
        let population_v4_digest = receipt.content_digest().expect("Population V4 digest");
        let admission_completion = AdmissionCompletionReceiptV1::derive(
            population_id,
            population_v4_digest,
            &admission_policy,
            &decisions,
        )
        .expect("canonical admission completion");
        let fingerprint = column
            .evaluation_spec_token()
            .expect("integration column carries evaluator identity")
            .fingerprint_v1();
        let parameters = [
            canonical_execution_parameters_fixture_v1(
                population_id,
                population_v4_digest,
                TradeDirectionV1::Long,
                instrument_family,
                rung_seconds,
                execution_horizon(),
                run_params(),
                fingerprint,
                &long_grid,
            )
            .expect("canonical long execution parameters"),
            canonical_execution_parameters_fixture_v1(
                population_id,
                population_v4_digest,
                TradeDirectionV1::Short,
                instrument_family,
                rung_seconds,
                execution_horizon(),
                run_params(),
                fingerprint,
                &short_grid,
            )
            .expect("canonical short execution parameters"),
        ];
        let dispositions = rows
            .iter()
            .copied()
            .zip(&decisions)
            .zip(&classifications)
            .map(|((row, decision), classification)| {
                let parameters = match row.direction {
                    TradeDirectionV1::Long => &parameters[0],
                    TradeDirectionV1::Short => &parameters[1],
                };
                let capability = classification
                    .selected()
                    .map(|selected| ExecutionStrategyCapabilityV1::new(parameters, row, selected))
                    .transpose()
                    .expect("authorized fixture capability");
                ExecutionRowDispositionV2::from_classification(
                    &receipt,
                    &admission_completion,
                    row,
                    decision,
                    parameters,
                    classification,
                    capability,
                )
                .expect("canonical row disposition")
            })
            .collect::<Vec<_>>();

        let mut population = PopulationLedger::open(store_root).expect("open Population V4 ledger");
        assert!(matches!(
            population
                .append_complete_v4(&rows, &receipt)
                .expect("commit Population V4"),
            PopulationCommit::Written | PopulationCommit::Reused
        ));
        let mut admission =
            AdmissionAuthorityLedger::open(store_root).expect("open admission authority ledger");
        let committed_admission = admission
            .commit(
                population_id,
                population_v4_digest,
                &admission_policy,
                &decisions,
            )
            .expect("commit admission authority");
        let committed_receipt = match committed_admission {
            crate::admission_store::AdmissionCommitOutcomeV1::Appended(value)
            | crate::admission_store::AdmissionCommitOutcomeV1::Reused(value) => value,
        };
        assert_eq!(committed_receipt, admission_completion);
        let prepared = PreparedExecutionDispositionsV2::from_ledgers(
            &mut population,
            &mut admission,
            population_id,
            parameters[0].clone(),
            parameters[1].clone(),
            dispositions.clone(),
        )
        .expect("public V2 preparation joins both committed authorities");

        CanonicalExecutionDispositionV2Fixture {
            receipt,
            admission_completion,
            admission_policy,
            ranking_policy,
            rows,
            decisions,
            parameters,
            classifications,
            dispositions,
            prepared,
            training,
            instrument,
            column,
            grids: [long_grid, short_grid],
            training_runs: [long_run, short_run],
            mask_words,
        }
    }

    fn row(
        population_id: [u8; 32],
        population_v4_digest: [u8; 32],
        admission_completion_digest: [u8; 32],
        parameters: &ExecutionParametersV1,
        sequence: u64,
        admission_status: AdmissionStatusV1,
        tag: ExecutionDispositionTagV2,
        refusal_bits: ExecutionRefusalBitsV1,
    ) -> ExecutionRowDispositionV2 {
        let seed = u8::try_from(sequence).unwrap_or(u8::MAX).wrapping_add(40);
        let mut value = ExecutionRowDispositionV2 {
            disposition_id: [0; 32],
            population_id,
            population_v4_digest,
            admission_completion_digest,
            row_payload_digest: digest(seed),
            strategy_digest: digest(seed.wrapping_add(1)),
            capability_id: match tag {
                ExecutionDispositionTagV2::Authorized => digest(seed.wrapping_add(2)),
                ExecutionDispositionTagV2::PolicyRefused => [0; 32],
            },
            parameter_id: parameters.parameter_id(),
            runner_disposition_digest: digest(seed.wrapping_add(3)),
            resolution_digest: parameters.expected_resolved_digest(),
            training_run_id: digest(seed.wrapping_add(4)),
            context_digest: digest(seed.wrapping_add(5)),
            column_digest: digest(seed.wrapping_add(6)),
            row_sequence: sequence,
            refusal_bits,
            admission_status,
            tag,
            coordinate: Chosen {
                stop: Some(usize::try_from(sequence).expect("fixture coordinate")),
                target: Some(2),
                tsl: None,
                ttp: Some(Ttp { arm: 1, trail: 1 }),
            },
        };
        value.disposition_id = value.derived_id().expect("fixture disposition identity");
        value.validate().expect("valid fixture disposition");
        value
    }

    fn prepared_for(population_seed: u8) -> PreparedExecutionDispositionsV2 {
        let population_id = digest(population_seed);
        let population_v4_digest = digest(population_seed.wrapping_add(1));
        let admission_completion_digest = digest(population_seed.wrapping_add(2));
        let parameters = parameter_pair(population_id, population_v4_digest);
        let scalars = [
            ParameterScalarV1::from_parameters(&parameters[0]).expect("long scalar"),
            ParameterScalarV1::from_parameters(&parameters[1]).expect("short scalar"),
        ];
        let mut percentiles =
            PercentileRecordV1::from_parameters(&parameters[0]).expect("long percentiles");
        percentiles.extend(
            PercentileRecordV1::from_parameters(&parameters[1]).expect("short percentiles"),
        );
        let specifications = [
            (
                0_usize,
                AdmissionStatusV1::Unmeasured,
                ExecutionDispositionTagV2::Authorized,
                ExecutionRefusalBitsV1::NONE,
            ),
            (
                1,
                AdmissionStatusV1::Unmeasured,
                ExecutionDispositionTagV2::PolicyRefused,
                ExecutionRefusalBitsV1::ZERO_TRADES,
            ),
            (
                0,
                AdmissionStatusV1::Refused,
                ExecutionDispositionTagV2::Authorized,
                ExecutionRefusalBitsV1::NONE,
            ),
            (
                1,
                AdmissionStatusV1::Refused,
                ExecutionDispositionTagV2::PolicyRefused,
                ExecutionRefusalBitsV1::MISSING_TARGET,
            ),
        ];
        let rows = specifications
            .into_iter()
            .enumerate()
            .map(|(sequence, (parameter, admission, tag, bits))| {
                row(
                    population_id,
                    population_v4_digest,
                    admission_completion_digest,
                    &parameters[parameter],
                    u64::try_from(sequence).expect("fixture sequence"),
                    admission,
                    tag,
                    bits,
                )
            })
            .collect::<Vec<_>>();
        let mut matrix = AdmissionExecutionMatrixV2::ZERO;
        let mut ordered = brutex_core::blake3::Hasher::new();
        ordered.update(ORDERED_ROW_DOMAIN);
        ordered.update(
            &u64::try_from(rows.len())
                .expect("fixture row count")
                .to_le_bytes(),
        );
        for disposition in &rows {
            matrix
                .push(disposition.admission_status, disposition.tag)
                .expect("fixture matrix");
            ordered.update(&disposition.to_bytes().expect("fixture row bytes"));
        }
        PreparedExecutionDispositionsV2 {
            population_id,
            population_v4_digest,
            admission_completion_digest,
            ordered_parameter_digest: parameter_digest_records(&scalars)
                .expect("fixture parameter digest"),
            ordered_percentile_digest: percentile_digest_records(&percentiles)
                .expect("fixture percentile digest"),
            parameters,
            scalars,
            percentiles,
            row_count: u64::try_from(rows.len()).expect("fixture row count"),
            authorized_capability_count: 2,
            policy_refused_count: 2,
            matrix,
            ordered_disposition_digest: ordered.finalize(),
            rows,
        }
    }

    fn root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-execution-disposition-v2-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    fn clean(path: &Path) {
        let _ignored = fs::remove_dir_all(path);
        fs::create_dir(path).expect("execution V2 authority root fixture creates");
    }

    fn reseal_row(raw: &mut [u8; EXECUTION_DISPOSITION_ROW_STRIDE_V2]) {
        let payload: [u8; ROW_PAYLOAD_BYTES] =
            raw[..ROW_PAYLOAD_BYTES].try_into().expect("row payload");
        raw[ROW_PAYLOAD_BYTES..].copy_from_slice(&brutex_core::blake3::hash(&payload));
    }

    fn reseal_completion(raw: &mut [u8; EXECUTION_DISPOSITION_COMPLETION_STRIDE_V2]) {
        let payload: [u8; COMPLETION_PAYLOAD_BYTES] = raw[..COMPLETION_PAYLOAD_BYTES]
            .try_into()
            .expect("completion payload");
        raw[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&brutex_core::blake3::hash(&payload));
    }

    #[test]
    fn private_matrix_kernel_covers_all_four_statuses_and_both_execution_terminals() {
        let mut matrix = AdmissionExecutionMatrixV2::ZERO;
        for admission in [
            AdmissionStatusV1::Admitted,
            AdmissionStatusV1::Rejected,
            AdmissionStatusV1::Unmeasured,
            AdmissionStatusV1::Refused,
        ] {
            for execution in [
                ExecutionDispositionTagV2::Authorized,
                ExecutionDispositionTagV2::PolicyRefused,
            ] {
                matrix
                    .push(admission, execution)
                    .expect("private matrix cell increments exactly once");
                assert_eq!(matrix.count(admission, execution), 1);
            }
        }
        assert_eq!(matrix.total().expect("bounded private matrix total"), 8);
    }

    #[test]
    fn public_ledgers_preserve_the_truthful_v1_block_and_reclassify_after_reopen() {
        let store_root = root("public-ledgers-reclassify");
        clean(&store_root);
        let fixture = canonical_execution_disposition_v2_fixture(
            &store_root,
            50,
            InstrumentFamilyV1::Nifty,
            60,
            0,
            RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy"),
        );
        assert_eq!(fixture.prepared.row_count(), 8);
        assert_eq!(fixture.prepared.authorized_capability_count(), 4);
        assert_eq!(fixture.prepared.policy_refused_count(), 4);
        let matrix = ExecutionCapabilityCompletionV2::for_prepared(&fixture.prepared, 0, 0, 0)
            .expect("semantic fixture completion")
            .admission_execution_matrix();
        assert_eq!(fixture.admission_completion.admitted_count(), 0);
        assert_eq!(fixture.admission_completion.rejected_count(), 0);
        assert_eq!(fixture.admission_completion.unmeasured_count(), 4);
        assert_eq!(fixture.admission_completion.refused_count(), 4);
        for admission in [AdmissionStatusV1::Unmeasured, AdmissionStatusV1::Refused] {
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::Authorized),
                2
            );
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::PolicyRefused),
                2
            );
        }
        for admission in [AdmissionStatusV1::Admitted, AdmissionStatusV1::Rejected] {
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::Authorized),
                0
            );
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::PolicyRefused),
                0
            );
        }

        let mut population =
            PopulationLedger::open(&store_root).expect("reopen Population V4 authority");
        let mut admission =
            AdmissionAuthorityLedger::open(&store_root).expect("reopen admission authority");
        let mut missing = fixture.dispositions.clone();
        assert!(missing.pop().is_some());
        assert!(
            PreparedExecutionDispositionsV2::from_ledgers(
                &mut population,
                &mut admission,
                fixture.receipt.population_id(),
                fixture.parameters[0].clone(),
                fixture.parameters[1].clone(),
                missing,
            )
            .expect_err("one missing terminal disposition must refuse")
            .contains("received 7 dispositions for 8 Population V4 rows")
        );
        drop(admission);
        drop(population);

        let mut ledger = ExecutionDispositionLedgerV2::open(&store_root)
            .expect("open execution disposition ledger");
        assert!(matches!(
            ledger
                .append_complete(&fixture.prepared)
                .expect("append public-path V2 authority"),
            ExecutionDispositionCommitV2::Written(_)
        ));
        drop(ledger);
        let mut reopened = ExecutionDispositionLedgerV2::open_read(&store_root)
            .expect("reopen execution disposition ledger");
        for index in 0..fixture.rows.len() {
            let row = fixture.rows[index];
            let parameters = match row.direction {
                TradeDirectionV1::Long => &fixture.parameters[0],
                TradeDirectionV1::Short => &fixture.parameters[1],
            };
            let stored = reopened
                .row(row.population_id, row.sequence)
                .expect("generation-checked row lookup")
                .expect("durable row exists");
            let fresh = stored
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    row,
                    &fixture.decisions[index],
                    parameters,
                    &fixture.classifications[index],
                )
                .expect("fresh evidence reproduces durable terminal");
            assert_eq!(
                matches!(fresh, ReclassifiedExecutionDispositionV2::Authorized(_)),
                stored.is_authorized()
            );
        }
        drop(reopened);
        fs::remove_dir_all(store_root).expect("remove public-path fixture");
    }

    #[test]
    fn parameterized_fixture_proves_execution_capacity_but_v1_blocks_top25_admission() {
        let store_root = root("banknifty-hourly-top25");
        clean(&store_root);
        let fixture = canonical_execution_disposition_v2_fixture(
            &store_root,
            51,
            InstrumentFamilyV1::BankNifty,
            3_600,
            25,
            RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy"),
        );
        let matrix = ExecutionCapabilityCompletionV2::for_prepared(&fixture.prepared, 0, 0, 0)
            .expect("semantic fixture completion")
            .admission_execution_matrix();
        assert!(fixture.prepared.authorized_capability_count() >= 25);
        assert_eq!(fixture.admission_completion.admitted_count(), 0);
        assert_eq!(fixture.admission_completion.rejected_count(), 0);
        for admission in [AdmissionStatusV1::Unmeasured, AdmissionStatusV1::Refused] {
            assert!(
                matrix.count(admission, ExecutionDispositionTagV2::Authorized) > 0,
                "all admission/execution matrix cells remain represented"
            );
            assert!(
                matrix.count(admission, ExecutionDispositionTagV2::PolicyRefused) > 0,
                "all admission/execution matrix cells remain represented"
            );
        }
        for admission in [AdmissionStatusV1::Admitted, AdmissionStatusV1::Rejected] {
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::Authorized),
                0
            );
            assert_eq!(
                matrix.count(admission, ExecutionDispositionTagV2::PolicyRefused),
                0
            );
        }
        assert!(fixture.rows.iter().all(|row| {
            row.instrument_family == InstrumentFamilyV1::BankNifty && row.rung_seconds == 3_600
        }));
        let identities = fixture
            .rows
            .iter()
            .map(|row| row.strategy_digest)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), fixture.rows.len());
        assert_eq!(fixture.dispositions.len(), fixture.rows.len());
        assert_eq!(
            fixture.ranking_policy.digest(),
            RankingPolicyV1::new(Weights::equal())
                .expect("equal ranking policy")
                .digest()
        );
        assert_ne!(fixture.training_runs[0], fixture.training_runs[1]);
        assert_eq!(
            fixture.training_series().bars(),
            fixture.training.as_slice()
        );
        fs::remove_dir_all(store_root).expect("remove parameterized fixture");
    }

    #[test]
    fn durable_reclassification_refuses_every_foreign_authority_axis() {
        let store_root = root("reclassification-attacks");
        clean(&store_root);
        let fixture = canonical_execution_disposition_v2_fixture(
            &store_root,
            52,
            InstrumentFamilyV1::Nifty,
            60,
            0,
            RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy"),
        );
        let mut ledger = ExecutionDispositionLedgerV2::open(&store_root)
            .expect("open execution disposition ledger");
        ledger
            .append_complete(&fixture.prepared)
            .expect("append canonical V2 execution authority");
        let authorized_row = fixture.rows[0];
        let refused_row = fixture.rows[1];
        let authorized = ledger
            .row(authorized_row.population_id, authorized_row.sequence)
            .expect("authorized row lookup")
            .expect("authorized durable row");
        let refused = ledger
            .row(refused_row.population_id, refused_row.sequence)
            .expect("refused row lookup")
            .expect("refused durable row");
        assert!(authorized.is_authorized());
        assert!(refused.is_policy_refused());

        let mut foreign_run_params = run_params();
        foreign_run_params.policy = foreign_run_params.policy.saturating_add(1);
        let (cross_run_classifications, _) = classify_side(
            &fixture.grids[0],
            fixture.training_series(),
            &fixture.column,
            &fixture.training,
            fixture.mask_words,
            TradeDirectionV1::Long,
            foreign_run_params,
        );
        let cross_run = classification_at(
            &cross_run_classifications,
            fixture.classifications[0].coordinate(),
        );
        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    authorized_row,
                    &fixture.decisions[0],
                    &fixture.parameters[0],
                    &cross_run,
                )
                .is_err(),
            "a same-coordinate classification from another canonical run must refuse"
        );

        let mut foreign_thresholds = Thresholds::CLASSICAL;
        foreign_thresholds.doji_body = foreign_thresholds.doji_body.saturating_add(1);
        let foreign_column = integration_column(&fixture.training, foreign_thresholds);
        let (cross_fingerprint_classifications, _) = classify_side(
            &fixture.grids[0],
            fixture.training_series(),
            &foreign_column,
            &fixture.training,
            fixture.mask_words,
            TradeDirectionV1::Long,
            run_params(),
        );
        let cross_fingerprint = classification_at(
            &cross_fingerprint_classifications,
            fixture.classifications[0].coordinate(),
        );
        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    authorized_row,
                    &fixture.decisions[0],
                    &fixture.parameters[0],
                    &cross_fingerprint,
                )
                .is_err(),
            "a changed evaluator fingerprint must refuse"
        );

        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    authorized_row,
                    &fixture.decisions[0],
                    &fixture.parameters[1],
                    &fixture.classifications[0],
                )
                .is_err(),
            "a short-side parameter authority cannot reclassify a long row"
        );

        let (foreign_series, foreign_grid) = resolve_grid(
            Side::Long,
            RangeResolutionV1::PpmFloor,
            &fixture.instrument,
            &fixture.training,
        );
        let (foreign_grid_classifications, _) = classify_side(
            &foreign_grid,
            foreign_series,
            &fixture.column,
            &fixture.training,
            fixture.mask_words,
            TradeDirectionV1::Long,
            run_params(),
        );
        let cross_grid = classification_at(
            &foreign_grid_classifications,
            fixture.classifications[0].coordinate(),
        );
        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    authorized_row,
                    &fixture.decisions[0],
                    &fixture.parameters[0],
                    &cross_grid,
                )
                .is_err(),
            "a changed resolved grid/context must refuse"
        );

        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    authorized_row,
                    &fixture.decisions[0],
                    &fixture.parameters[0],
                    &fixture.classifications[1],
                )
                .is_err(),
            "a policy-refused terminal cannot replace durable authorization"
        );
        assert!(
            refused
                .require_reclassification(
                    &fixture.receipt,
                    &fixture.admission_completion,
                    refused_row,
                    &fixture.decisions[1],
                    &fixture.parameters[0],
                    &fixture.classifications[0],
                )
                .is_err(),
            "an authorized terminal cannot replace a durable policy refusal"
        );

        let foreign_admission_policy = admission_policy(11);
        let foreign_admission_decisions = fixture
            .rows
            .iter()
            .zip(&fixture.decisions)
            .map(|(row, decision)| {
                let sealed = foreign_admission_policy.evaluate_sealed(decision.evidence());
                assert_eq!(sealed.verdict().status(), decision.verdict().status());
                AdmissionDecisionRecordV1::new(
                    row.population_id,
                    row.sequence,
                    row.strategy_digest,
                    row.payload_digest().expect("canonical fixture row"),
                    &sealed,
                )
                .expect("foreign-policy decision remains canonical")
            })
            .collect::<Vec<_>>();
        let population_v4_digest = fixture
            .receipt
            .content_digest()
            .expect("Population V4 content digest");
        let foreign_admission_completion = AdmissionCompletionReceiptV1::derive(
            authorized_row.population_id,
            population_v4_digest,
            &foreign_admission_policy,
            &foreign_admission_decisions,
        )
        .expect("foreign admission completion");
        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &foreign_admission_completion,
                    authorized_row,
                    &foreign_admission_decisions[0],
                    &fixture.parameters[0],
                    &fixture.classifications[0],
                )
                .is_err(),
            "a different complete admission policy/receipt cannot borrow the row"
        );

        let changed_evidence = admission_evidence(AdmissionStatusV1::Refused);
        let changed_seal = fixture.admission_policy.evaluate_sealed(&changed_evidence);
        assert_eq!(changed_seal.verdict().status(), AdmissionStatusV1::Refused);
        let mut changed_row = authorized_row;
        changed_row.admission = project_admission(changed_seal.verdict());
        let changed_decision = AdmissionDecisionRecordV1::new(
            changed_row.population_id,
            changed_row.sequence,
            changed_row.strategy_digest,
            changed_row
                .payload_digest()
                .expect("changed row remains canonical"),
            &changed_seal,
        )
        .expect("changed-status admission decision");
        let mut changed_decisions = fixture.decisions.clone();
        changed_decisions[0] = changed_decision.clone();
        let changed_completion = AdmissionCompletionReceiptV1::derive(
            changed_row.population_id,
            population_v4_digest,
            &fixture.admission_policy,
            &changed_decisions,
        )
        .expect("changed-status admission completion");
        assert!(
            authorized
                .require_reclassification(
                    &fixture.receipt,
                    &changed_completion,
                    changed_row,
                    &changed_decision,
                    &fixture.parameters[0],
                    &fixture.classifications[0],
                )
                .is_err(),
            "a changed admission terminal and row payload must refuse"
        );
        drop(ledger);
        fs::remove_dir_all(store_root).expect("remove attack fixture");
    }

    #[test]
    fn fixed_row_codec_preserves_dynamic_provenance_and_both_terminals() {
        let prepared = prepared_for(1);
        for expected in prepared.rows.iter().copied() {
            let decoded = ExecutionRowDispositionV2::from_bytes(
                &expected.to_bytes().expect("encode disposition"),
            )
            .expect("decode disposition");
            assert_eq!(decoded, expected);
            assert_ne!(decoded.parameter_id(), [0; 32]);
            assert_ne!(decoded.runner_disposition_digest(), [0; 32]);
            assert_ne!(decoded.context_digest(), [0; 32]);
            assert_ne!(decoded.column_digest(), [0; 32]);
            assert_eq!(decoded.coordinate(), expected.coordinate());
            assert_eq!(decoded.is_authorized(), decoded.capability_id().is_some());
            assert_eq!(
                decoded.is_policy_refused(),
                !decoded.refusal_bits().is_empty()
            );
        }
    }

    #[test]
    fn row_decoder_refuses_seal_unknown_tags_bits_reserve_and_half_ttp() {
        let row = prepared_for(10).rows[0];
        let mut corrupt = row.to_bytes().expect("row bytes");
        corrupt[0] ^= 1;
        assert!(
            ExecutionRowDispositionV2::from_bytes(&corrupt)
                .expect_err("seal corruption")
                .contains("seal")
        );

        let mut unknown_bits = row.to_bytes().expect("row bytes");
        unknown_bits[424..432].copy_from_slice(&(1_u64 << 63).to_le_bytes());
        reseal_row(&mut unknown_bits);
        assert!(
            ExecutionRowDispositionV2::from_bytes(&unknown_bits)
                .expect_err("unknown bits")
                .contains("unknown")
        );

        let mut unknown_tag = row.to_bytes().expect("row bytes");
        unknown_tag[433] = 99;
        reseal_row(&mut unknown_tag);
        assert!(
            ExecutionRowDispositionV2::from_bytes(&unknown_tag)
                .expect_err("unknown terminal tag")
                .contains("tag")
        );

        let mut half_ttp = row.to_bytes().expect("row bytes");
        half_ttp[458..466].copy_from_slice(&u64::MAX.to_le_bytes());
        reseal_row(&mut half_ttp);
        assert!(
            ExecutionRowDispositionV2::from_bytes(&half_ttp)
                .expect_err("half-present TTP")
                .contains("only one")
        );

        let mut reserve = row.to_bytes().expect("row bytes");
        reserve[474] = 1;
        reseal_row(&mut reserve);
        assert!(
            ExecutionRowDispositionV2::from_bytes(&reserve)
                .expect_err("nonzero row reserve")
                .contains("reserve")
        );
    }

    #[test]
    fn completion_codec_binds_all_three_blocks_and_excludes_physical_offsets() {
        let prepared = prepared_for(20);
        let first = ExecutionCapabilityCompletionV2::for_prepared(&prepared, 0, 0, 0)
            .expect("first completion");
        let later = ExecutionCapabilityCompletionV2::for_prepared(&prepared, 7, 31, 101)
            .expect("completion after crash orphans");
        assert_eq!(first.completion_id(), later.completion_id());
        assert_ne!(
            first.to_bytes().expect("first bytes"),
            later.to_bytes().expect("later bytes")
        );
        assert_eq!(
            later.parameter_ids(),
            [
                prepared.parameters[0].parameter_id(),
                prepared.parameters[1].parameter_id(),
            ]
        );
        assert_eq!(
            ExecutionCapabilityCompletionV2::from_bytes(
                &later.to_bytes().expect("completion bytes")
            )
            .expect("decode completion"),
            later
        );

        let mut reserve = first.to_bytes().expect("completion bytes");
        reserve[456] = 1;
        reseal_completion(&mut reserve);
        assert!(
            ExecutionCapabilityCompletionV2::from_bytes(&reserve)
                .expect_err("completion reserve")
                .contains("reserve")
        );
    }

    #[test]
    fn ledger_appends_reopens_pages_reuses_and_reconstructs_parameters() {
        let root = root("append-reopen");
        clean(&root);
        let prepared = prepared_for(30);
        let expected_id = prepared.completion_id().expect("semantic completion");
        let mut ledger = ExecutionDispositionLedgerV2::open(&root).expect("open ledger");
        assert!(matches!(
            ledger.append_complete(&prepared).expect("append authority"),
            ExecutionDispositionCommitV2::Written(_)
        ));
        assert!(matches!(
            ledger.append_complete(&prepared).expect("exact reuse"),
            ExecutionDispositionCommitV2::Reused(_)
        ));
        assert_eq!(
            ledger
                .completion(prepared.population_id)
                .expect("completion lookup")
                .expect("stored completion")
                .completion_id(),
            expected_id
        );
        assert_eq!(
            ledger
                .parameters(prepared.population_id, TradeDirectionV1::Short)
                .expect("parameter lookup")
                .expect("stored short parameter"),
            prepared.parameters[1]
        );
        let page = ledger
            .page(prepared.population_id, 1, 2)
            .expect("page lookup")
            .expect("stored page");
        assert_eq!(
            page.iter().copied().collect::<Vec<_>>(),
            prepared.rows[1..3]
        );
        assert!(
            ledger
                .page(
                    prepared.population_id,
                    0,
                    MAX_EXECUTION_DISPOSITION_PAGE_ROWS_V2 + 1,
                )
                .expect_err("over-ceiling page")
                .contains("ceiling")
        );
        drop(ledger);

        let mut reopened = ExecutionDispositionLedgerV2::open_read(&root).expect("reopen ledger");
        assert_eq!(
            reopened
                .completion(prepared.population_id)
                .expect("reopen lookup")
                .expect("reopened completion")
                .completion_id(),
            expected_id
        );
        assert_eq!(
            reopened
                .page(prepared.population_id, 0, 4)
                .expect("reopened page")
                .expect("stored authority")
                .into_rows(),
            prepared.rows
        );
        drop(reopened);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn writable_ledger_refuses_missing_or_nondirectory_root_without_recreation() {
        let missing_root = root("missing-authority-root");
        let _ignored = fs::remove_dir_all(&missing_root);
        assert!(!missing_root.exists());
        let missing_error = ExecutionDispositionLedgerV2::open(&missing_root)
            .expect_err("a missing removable authority root must refuse");
        assert!(missing_error.contains("must already exist as a directory"));
        assert!(!missing_root.exists());

        let file_root = root("file-authority-root");
        let _ignored = fs::remove_dir_all(&file_root);
        File::create(&file_root).expect("non-directory authority fixture creates");
        let file_error = ExecutionDispositionLedgerV2::open(&file_root)
            .expect_err("a non-directory authority root must refuse");
        assert!(file_error.contains("is not a directory"));
        assert!(file_root.is_file());
        fs::remove_file(file_root).expect("non-directory authority fixture removes");
    }

    #[test]
    fn valid_orphans_survive_but_torn_files_and_same_length_corruption_refuse() {
        let orphan_root = root("valid-orphans");
        clean(&orphan_root);
        let prepared = prepared_for(40);
        let mut ledger = ExecutionDispositionLedgerV2::open(&orphan_root).expect("open ledger");
        ledger.append_complete(&prepared).expect("append authority");
        drop(ledger);
        for (path, bytes) in [
            (
                ExecutionDispositionLedgerV2::parameter_path(&orphan_root),
                prepared.scalars[0]
                    .to_bytes()
                    .expect("orphan scalar")
                    .to_vec(),
            ),
            (
                ExecutionDispositionLedgerV2::percentile_path(&orphan_root),
                prepared.percentiles[0]
                    .to_bytes()
                    .expect("orphan percentile")
                    .to_vec(),
            ),
            (
                ExecutionDispositionLedgerV2::row_path(&orphan_root),
                prepared.rows[0].to_bytes().expect("orphan row").to_vec(),
            ),
        ] {
            let mut file = OpenOptions::new()
                .append(true)
                .open(path)
                .expect("open orphan tail");
            file.write_all(&bytes).expect("append valid orphan");
            file.sync_all().expect("sync valid orphan");
        }
        drop(
            ExecutionDispositionLedgerV2::open_read(&orphan_root)
                .expect("valid unreferenced records remain reopenable"),
        );
        fs::remove_dir_all(orphan_root).expect("remove orphan fixture");

        for (tag, path_of) in [
            (
                "torn-parameter",
                ExecutionDispositionLedgerV2::parameter_path as fn(&Path) -> PathBuf,
            ),
            (
                "torn-percentile",
                ExecutionDispositionLedgerV2::percentile_path as fn(&Path) -> PathBuf,
            ),
            (
                "torn-row",
                ExecutionDispositionLedgerV2::row_path as fn(&Path) -> PathBuf,
            ),
            (
                "torn-completion",
                ExecutionDispositionLedgerV2::completion_path as fn(&Path) -> PathBuf,
            ),
        ] {
            let root = root(tag);
            clean(&root);
            drop(ExecutionDispositionLedgerV2::open(&root).expect("create ledger"));
            let mut file = OpenOptions::new()
                .append(true)
                .open(path_of(&root))
                .expect("open torn tail");
            file.write_all(&[1]).expect("append torn byte");
            file.sync_all().expect("sync torn byte");
            drop(file);
            assert!(
                ExecutionDispositionLedgerV2::open_read(&root)
                    .expect_err("torn fixed file")
                    .contains("ragged/torn")
            );
            fs::remove_dir_all(root).expect("remove torn fixture");
        }

        let corrupt_root = root("same-length-corruption");
        clean(&corrupt_root);
        let prepared = prepared_for(50);
        let mut ledger =
            ExecutionDispositionLedgerV2::open(&corrupt_root).expect("open corrupt fixture");
        ledger.append_complete(&prepared).expect("append authority");
        drop(ledger);
        let path = ExecutionDispositionLedgerV2::completion_path(&corrupt_root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open completion for corruption");
        file.seek(SeekFrom::Start(HEADER_BYTES + 5))
            .expect("seek corruption");
        file.write_all(&[0xA5]).expect("write corruption");
        file.sync_all().expect("sync corruption");
        drop(file);
        assert!(
            ExecutionDispositionLedgerV2::open_read(&corrupt_root)
                .expect_err("same-length corruption")
                .contains("seal")
        );
        fs::remove_dir_all(corrupt_root).expect("remove corrupt fixture");

        let matrix_root = root("rekeyed-matrix-redistribution");
        clean(&matrix_root);
        let prepared = prepared_for(51);
        let mut ledger =
            ExecutionDispositionLedgerV2::open(&matrix_root).expect("open matrix fixture");
        ledger
            .append_complete(&prepared)
            .expect("append matrix authority");
        drop(ledger);
        let mut forged = ExecutionCapabilityCompletionV2::for_prepared(&prepared, 0, 0, 0)
            .expect("canonical matrix completion");
        assert_eq!(forged.matrix.counts[4], 1);
        assert_eq!(forged.matrix.counts[6], 1);
        forged.matrix.counts[4] = 2;
        forged.matrix.counts[6] = 0;
        forged.completion_id = forged.derived_id();
        let forged_bytes = forged
            .to_bytes()
            .expect("redistributed matrix remains internally canonical");
        let completion_path = ExecutionDispositionLedgerV2::completion_path(&matrix_root);
        let mut file = OpenOptions::new()
            .write(true)
            .open(completion_path)
            .expect("open completion for matrix redistribution");
        file.seek(SeekFrom::Start(HEADER_BYTES))
            .expect("seek matrix completion");
        file.write_all(&forged_bytes)
            .expect("write rekeyed matrix completion");
        file.sync_all().expect("sync rekeyed matrix completion");
        drop(file);
        assert!(
            ExecutionDispositionLedgerV2::open_read(&matrix_root)
                .expect_err("row-derived matrix must defeat a rekeyed redistribution")
                .contains("counts, matrix or ordered digest")
        );
        fs::remove_dir_all(matrix_root).expect("remove matrix fixture");

        let stale_root = root("stale-same-length-handle");
        clean(&stale_root);
        let prepared = prepared_for(52);
        let mut ledger =
            ExecutionDispositionLedgerV2::open(&stale_root).expect("open stale fixture");
        ledger
            .append_complete(&prepared)
            .expect("append stale-handle authority");
        let row_path = ExecutionDispositionLedgerV2::row_path(&stale_root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(row_path)
            .expect("open row file for same-length mutation");
        let at = HEADER_BYTES + 5;
        file.seek(SeekFrom::Start(at))
            .expect("seek stale mutation byte");
        let mut original = [0_u8; 1];
        file.read_exact(&mut original)
            .expect("read stale mutation byte");
        file.seek(SeekFrom::Start(at))
            .expect("rewind stale mutation byte");
        file.write_all(&[original[0] ^ 0xFF])
            .expect("write same-length stale mutation");
        file.sync_all().expect("sync same-length stale mutation");
        drop(file);
        assert!(
            ledger
                .completion(prepared.population_id)
                .expect_err("an open handle must refuse post-open same-length mutation")
                .contains("changed after open")
        );
        drop(ledger);
        fs::remove_dir_all(stale_root).expect("remove stale fixture");
    }
}
