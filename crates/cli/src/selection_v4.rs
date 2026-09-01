//! Execution-authoritative global Top-25 receipts for one signal timeframe.
//!
//! Version four is an append-only successor to selection V3. It has its own
//! magic, path, header, stride and identity domain and never reads an earlier
//! selection format. A V4 receipt binds the exact Population V4 completion,
//! receipt-last institutional-admission completion and Execution V2
//! disposition completion for both NIFTY and BANKNIFTY.
//!
//! The final eligibility predicate is deliberately a conjunction:
//! `institutional admission == admitted && execution disposition == authorized`.
//! Every population row is still visited and charged to a fixed 4-by-2
//! admission/execution accounting matrix. Noneligible rows participate in the
//! ordered proof, but do not define score extrema and cannot enter Top-N. In
//! particular, arbitrarily large metrics on a policy-refused execution row do
//! not change normalisation or the winners.
//!
//! # Reader integration boundary
//!
//! A crate-internal immutable adapter joins the concrete Population V4,
//! admission and Execution V2 authorities. It is not an external authoring
//! contract. The adapter must return pages already read under one authority
//! snapshot; this module independently rechecks every population, sequence,
//! strategy, payload and completion binding before using a row.
//!
//! # Cost
//!
//! Construction is three O(NIFTY rows + BANKNIFTY rows) bounded-page passes.
//! RAM is bounded by one 256-row page, 25 retained candidates and fixed proof
//! state. Ledger open is O(receipts). Exact and latest lookup after open are one
//! average-O(1) `HashMap` probe; Rust's `HashMap` does not provide a worst-case
//! O(1) guarantee. Construction, hashing, persistence and reopen are not O(1).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use costs::fill::Direction;
use runner::admission::AdmissionStatusV1;
use runner::portfolio::StrategyDigest;
use runner::topn::{
    Candidate, Extrema, MAX_TOP, Metrics, PopulationPass, PopulationProof, RankingPolicyV1,
    SCORE_SCALE, Selection, VerifiedKeeper,
};

use crate::population::{
    InstrumentFamilyV1, MAX_PAGE_ROWS_V1, PopulationRowV1, TopMetricsV1, TradeDirectionV1,
};
use crate::selection::{
    SELECTED_ENTRY_CANONICAL_LEN_V1, SHARED_COHORT_CANONICAL_LEN_V2, SelectedEntryV1,
    SharedCohortIdentityV2,
};

/// Operator-facing refusal from Selection V4 derivation or persistence.
pub type SelectionV4Refusal = String;

const MAGIC_V4: [u8; 8] = *b"BRUTXSL4";
const VERSION_V4: u32 = 4;
const HEADER_BYTES: usize = 40;
const HEADER_BYTES_U64: u64 = 40;
const HEADER_BYTES_U32: u32 = 40;
const SEAL_BYTES: usize = 32;
const REQUESTED_TOP_V4: u32 = 25;
const ACCOUNTING_CELLS: usize = 8;
const ACCOUNTING_BYTES_V4: usize = ACCOUNTING_CELLS * 8;
const POPULATION_REFERENCE_BYTES_V4: usize = 272;
const PAYLOAD_BYTES_V4: usize = 3_336;
const STRIDE_BYTES_U32_V4: u32 = 3_368;
const ID_DOMAIN_V4: &[u8] = b"brutex-global-selection-id-v4\0";
const JOINED_ORDER_DOMAIN_V4: &[u8] = b"brutex-selection-v4-joined-row-order\0";
const CANONICAL_RUNGS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];

/// Bytes in one canonical V4 population reference.
pub const POPULATION_REFERENCE_CANONICAL_LEN_V4: usize = POPULATION_REFERENCE_BYTES_V4;
/// Bytes in the sealed V4 receipt payload, excluding its BLAKE3 seal.
pub const SELECTION_V4_PAYLOAD_BYTES: usize = PAYLOAD_BYTES_V4;
/// Bytes in one complete sealed V4 selection receipt.
pub const SELECTION_V4_STRIDE_BYTES: usize = PAYLOAD_BYTES_V4 + SEAL_BYTES;
/// Fixed on-disk stride of one V4 selection receipt.
pub const SELECTION_V4_STRIDE: u64 = 3_368;

const _: () = assert!(MAX_TOP == 25);
const _: () = assert!(HEADER_BYTES == 40);
const _: () = assert!(HEADER_BYTES_U64 == 40);
const _: () = assert!(SHARED_COHORT_CANONICAL_LEN_V2 == 440);
const _: () = assert!(SELECTED_ENTRY_CANONICAL_LEN_V1 == 88);
const _: () = assert!(ACCOUNTING_BYTES_V4 == 64);
const _: () = assert!(POPULATION_REFERENCE_BYTES_V4 == 272);
const _: () = assert!(
    32 + 4
        + 4
        + (2 * POPULATION_REFERENCE_BYTES_V4)
        + (4 * 8)
        + 32
        + 32
        + SHARED_COHORT_CANONICAL_LEN_V2
        + 4
        + 4
        + (MAX_TOP * SELECTED_ENTRY_CANONICAL_LEN_V1)
        + 8
        == PAYLOAD_BYTES_V4
);
const _: () = assert!(PAYLOAD_BYTES_V4 + SEAL_BYTES == SELECTION_V4_STRIDE_BYTES);
const _: () = assert!(SELECTION_V4_STRIDE_BYTES == 3_368);
const _: () = assert!(SELECTION_V4_STRIDE == 3_368);
const _: () = assert!(STRIDE_BYTES_U32_V4 == 3_368);

/// Terminal Execution V2 classification used by selection.
///
/// The concrete Execution V2 adapter must map its sealed disposition into one
/// of these two exhaustive terminal classes and refuse every unknown value.
/// Structural failure, unmeasured evidence and upstream refusal are hard
/// errors, never successful persisted dispositions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionSelectionStatusV2 {
    /// Exact execution reconstruction and policy authorization succeeded.
    Authorized,
    /// Measured execution evidence violated the locked execution policy.
    PolicyRefused,
}

/// Exact terminal counts from an admission completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdmissionAuthorityCountsV4 {
    /// Rows admitted by the institutional policy.
    pub admitted: u64,
    /// Rows rejected by measured policy evidence.
    pub rejected: u64,
    /// Rows lacking required measured evidence.
    pub unmeasured: u64,
    /// Rows explicitly refused upstream.
    pub refused: u64,
}

impl AdmissionAuthorityCountsV4 {
    fn total(self) -> Result<u64, SelectionV4Refusal> {
        checked_sum(
            [self.admitted, self.rejected, self.unmeasured, self.refused],
            "admission authority counts",
        )
    }

    const fn by_index(self, index: usize) -> u64 {
        match index {
            0 => self.admitted,
            1 => self.rejected,
            2 => self.unmeasured,
            3 => self.refused,
            _ => 0,
        }
    }
}

/// Exact terminal counts from an Execution V2 completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExecutionAuthorityCountsV2 {
    /// Rows whose exact execution was authorized.
    pub authorized: u64,
    /// Rows rejected by measured execution policy.
    pub policy_refused: u64,
}

impl ExecutionAuthorityCountsV2 {
    fn total(self) -> Result<u64, SelectionV4Refusal> {
        checked_sum(
            [self.authorized, self.policy_refused],
            "execution authority counts",
        )
    }

    const fn by_index(self, index: usize) -> u64 {
        match index {
            0 => self.authorized,
            1 => self.policy_refused,
            _ => 0,
        }
    }
}

/// Immutable Population V4 facts applying to every page from one view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationAuthoritySnapshotV4 {
    /// NIFTY or BANKNIFTY in canonical global order.
    pub family: InstrumentFamilyV1,
    /// Exact complete population identity.
    pub population_id: [u8; 32],
    /// Shared signal timeframe.
    pub rung_seconds: u32,
    /// Exact number of contiguous V4 rows.
    pub row_count: u64,
    /// Ordered digest from the Population V4 completion lineage.
    pub ordered_row_digest: [u8; 32],
    /// Digest of the exact Population V4 completion receipt.
    pub completion_digest: [u8; 32],
    /// Ranking policy carried by Population V4.
    pub ranking_policy_digest: [u8; 32],
    /// Complete shared calendar/exit-policy cohort identity.
    pub cohort: SharedCohortIdentityV2,
}

/// Immutable receipt-last admission facts applying to one population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionAuthoritySnapshotV4 {
    /// Population covered by every decision.
    pub population_id: [u8; 32],
    /// Exact Population V4 completion digest this authority extends.
    pub population_v4_completion_digest: [u8; 32],
    /// Digest of the exact admission completion.
    pub completion_digest: [u8; 32],
    /// Number of sealed decisions; must equal the population row count.
    pub decision_count: u64,
    /// Exhaustive terminal decision counts.
    pub counts: AdmissionAuthorityCountsV4,
}

/// Immutable receipt-last Execution V2 facts applying to one population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionAuthoritySnapshotV2 {
    /// Population covered by every disposition.
    pub population_id: [u8; 32],
    /// Exact Population V4 completion digest this authority extends.
    pub population_v4_completion_digest: [u8; 32],
    /// Exact admission completion digest this authority extends.
    pub admission_completion_digest: [u8; 32],
    /// Digest of the exact Execution V2 completion.
    pub completion_digest: [u8; 32],
    /// Number of dispositions; must equal the population row count.
    pub disposition_count: u64,
    /// Exhaustive terminal disposition counts.
    pub counts: ExecutionAuthorityCountsV2,
    /// Exact four-admission-by-two-execution matrix sealed by the V2 completion.
    pub admission_execution_counts: [[u64; 2]; 4],
}

/// One immutable triple-authority snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionAuthoritySnapshotV4 {
    /// Population V4 authority.
    pub population: PopulationAuthoritySnapshotV4,
    /// Institutional-admission authority.
    pub admission: AdmissionAuthoritySnapshotV4,
    /// Exact-execution disposition authority.
    pub execution: ExecutionAuthoritySnapshotV2,
}

impl SelectionAuthoritySnapshotV4 {
    fn validate(&self, expected_family: InstrumentFamilyV1) -> Result<(), SelectionV4Refusal> {
        let population = &self.population;
        if population.family != expected_family {
            return Err(format!(
                "selection V4 family order is noncanonical: expected {expected_family:?}, found {:?}",
                population.family
            ));
        }
        if !CANONICAL_RUNGS.contains(&population.rung_seconds) {
            return Err(format!(
                "selection V4 timeframe {} seconds is outside the eight canonical intraday rungs",
                population.rung_seconds
            ));
        }
        self.validate_digests_and_cohort()?;
        self.validate_authority_bindings()?;
        self.validate_counts()
    }

    fn validate_digests_and_cohort(&self) -> Result<(), SelectionV4Refusal> {
        let population = &self.population;
        for (name, digest) in [
            ("population identity", population.population_id),
            (
                "ordered population-row digest",
                population.ordered_row_digest,
            ),
            (
                "Population V4 completion digest",
                population.completion_digest,
            ),
            ("ranking-policy digest", population.ranking_policy_digest),
            (
                "admission completion digest",
                self.admission.completion_digest,
            ),
            (
                "Execution V2 completion digest",
                self.execution.completion_digest,
            ),
            (
                "Execution V2 admission binding",
                self.execution.admission_completion_digest,
            ),
        ] {
            require_digest(name, &digest)?;
        }
        population.cohort.validate()?;
        if population.cohort.ranking_policy_digest() != population.ranking_policy_digest {
            return Err(
                "selection V4 population ranking policy differs from its cohort".to_owned(),
            );
        }
        Ok(())
    }

    fn validate_authority_bindings(&self) -> Result<(), SelectionV4Refusal> {
        let population = &self.population;
        if self.admission.population_id != population.population_id
            || self.execution.population_id != population.population_id
        {
            return Err(
                "selection V4 admission/execution authorities name a different population"
                    .to_owned(),
            );
        }
        if self.admission.population_v4_completion_digest != population.completion_digest
            || self.execution.population_v4_completion_digest != population.completion_digest
        {
            return Err(
                "selection V4 admission/execution authorities bind a different Population V4 completion"
                    .to_owned(),
            );
        }
        if self.execution.admission_completion_digest != self.admission.completion_digest {
            return Err(
                "selection V4 execution authority binds a different admission completion"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn validate_counts(&self) -> Result<(), SelectionV4Refusal> {
        let population = &self.population;
        if self.admission.decision_count != population.row_count
            || self.execution.disposition_count != population.row_count
        {
            return Err(
                "selection V4 does not have exactly one admission decision and one execution disposition per population row"
                    .to_owned(),
            );
        }
        if self.admission.counts.total()? != population.row_count {
            return Err(
                "selection V4 admission terminal counts do not cover the population".to_owned(),
            );
        }
        if self.execution.counts.total()? != population.row_count {
            return Err(
                "selection V4 execution terminal counts do not cover the population".to_owned(),
            );
        }
        let sealed_matrix = SelectionAccountingV4 {
            cells: self.execution.admission_execution_counts,
        };
        if sealed_matrix.total()? != population.row_count {
            return Err(
                "selection V4 Execution V2 matrix does not cover the population".to_owned(),
            );
        }
        for index in 0..4 {
            if sealed_matrix.admission_marginal(index)? != self.admission.counts.by_index(index) {
                return Err(
                    "selection V4 Execution V2 matrix disagrees with admission completion counts"
                        .to_owned(),
                );
            }
        }
        for index in 0..2 {
            if sealed_matrix.execution_marginal(index)? != self.execution.counts.by_index(index) {
                return Err(
                    "selection V4 Execution V2 matrix disagrees with execution completion counts"
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
}

/// One admission record reduced to the exact fields needed by selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionSelectionRowV4 {
    /// Population bound by the sealed decision.
    pub population_id: [u8; 32],
    /// Exact population sequence bound by the decision.
    pub sequence: u64,
    /// Exact semantic strategy bound by the decision.
    pub strategy_digest: [u8; 32],
    /// Digest of the canonical population payload bound by the decision.
    pub population_payload_digest: [u8; 32],
    /// Digest of the exact sealed admission decision.
    pub decision_digest: [u8; 32],
    /// Recomputed receipt-authorized institutional status.
    pub status: AdmissionStatusV1,
}

/// One Execution V2 record reduced to the exact fields needed by selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionSelectionRowV2 {
    /// Population bound by the sealed disposition.
    pub population_id: [u8; 32],
    /// Exact population sequence bound by the disposition.
    pub sequence: u64,
    /// Exact semantic strategy bound by the disposition.
    pub strategy_digest: [u8; 32],
    /// Digest of the canonical population payload bound by the disposition.
    pub population_payload_digest: [u8; 32],
    /// Population V4 completion digest sealed into this disposition.
    pub population_v4_completion_digest: [u8; 32],
    /// Admission completion digest sealed into this disposition.
    pub admission_completion_digest: [u8; 32],
    /// Admission terminal status sealed into this disposition.
    pub admission_status: AdmissionStatusV1,
    /// Digest of the exact sealed Execution V2 disposition.
    pub disposition_digest: [u8; 32],
    /// Receipt-authorized terminal execution classification.
    pub status: ExecutionSelectionStatusV2,
}

/// One exact Population V4 row joined to both independent authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionJoinedRowV4 {
    /// Exact Population V4 row.
    pub population: PopulationRowV1,
    /// Receipt-authorized institutional decision.
    pub admission: AdmissionSelectionRowV4,
    /// Receipt-authorized Execution V2 disposition.
    pub execution: ExecutionSelectionRowV2,
}

/// One bounded page read under one immutable triple-authority snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionAuthorityPageV4 {
    /// Total rows proved by all three completion receipts.
    pub total: u64,
    /// Requested zero-based population offset.
    pub offset: u64,
    /// Contiguous rows in canonical population order.
    pub rows: Vec<SelectionJoinedRowV4>,
    /// Population completion applying to every row.
    pub population_v4_completion_digest: [u8; 32],
    /// Admission completion applying to every row.
    pub admission_completion_digest: [u8; 32],
    /// Execution V2 completion applying to every row.
    pub execution_completion_digest: [u8; 32],
}

/// Crate-internal bounded view over one Population V4/admission/Execution V2 join.
///
/// Keeping this trait crate-private prevents callers from substituting a
/// hand-forged authority implementation for the durable three-ledger adapter.
pub(crate) trait SelectionAuthorityViewV4 {
    /// Immutable open-time snapshot rechecked by every page implementation.
    fn snapshot(&self) -> &SelectionAuthoritySnapshotV4;

    /// Returns exactly `min(limit, total - offset)` contiguous joined rows.
    ///
    /// # Errors
    ///
    /// Implementations must fail closed on a stale handle, changed completion,
    /// torn file, unknown disposition or any inability to hold one shared read
    /// generation across the three sources.
    fn page(
        &mut self,
        offset: u64,
        limit: u64,
    ) -> Result<SelectionAuthorityPageV4, SelectionV4Refusal>;
}

/// Complete cross-product accounting for one population.
///
/// Rows are institutional `[admitted, rejected, unmeasured, refused]`; columns
/// are execution `[authorized, policy-refused]`. Every nonterminal execution
/// condition is a hard error before a matrix can exist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionAccountingV4 {
    cells: [[u64; 2]; 4],
}

impl SelectionAccountingV4 {
    /// Count for one exact authority-status pair.
    #[must_use]
    pub fn count(self, admission: AdmissionStatusV1, execution: ExecutionSelectionStatusV2) -> u64 {
        self.cells
            .get(admission_index(admission))
            .and_then(|row| row.get(execution_index(execution)))
            .copied()
            .unwrap_or(0)
    }

    /// Rows authorized by both independent policies.
    #[must_use]
    pub fn eligible(self) -> u64 {
        self.count(
            AdmissionStatusV1::Admitted,
            ExecutionSelectionStatusV2::Authorized,
        )
    }

    /// Every row charged to this matrix.
    ///
    /// # Errors
    ///
    /// Refuses a counter sum that overflows `u64`.
    pub fn total(self) -> Result<u64, SelectionV4Refusal> {
        self.cells
            .into_iter()
            .flatten()
            .try_fold(0_u64, |sum, value| {
                sum.checked_add(value)
                    .ok_or_else(|| "selection V4 accounting total overflowed u64".to_owned())
            })
    }

    fn admission_marginal(self, index: usize) -> Result<u64, SelectionV4Refusal> {
        self.cells
            .get(index)
            .copied()
            .ok_or_else(|| "selection V4 admission accounting index is absent".to_owned())?
            .into_iter()
            .try_fold(0_u64, |sum, value| {
                sum.checked_add(value).ok_or_else(|| {
                    "selection V4 admission accounting marginal overflowed u64".to_owned()
                })
            })
    }

    fn execution_marginal(self, index: usize) -> Result<u64, SelectionV4Refusal> {
        self.cells.iter().try_fold(0_u64, |sum, row| {
            sum.checked_add(row.get(index).copied().unwrap_or(0))
                .ok_or_else(|| {
                    "selection V4 execution accounting marginal overflowed u64".to_owned()
                })
        })
    }

    fn charge(
        &mut self,
        admission: AdmissionStatusV1,
        execution: ExecutionSelectionStatusV2,
    ) -> Result<(), SelectionV4Refusal> {
        let slot = self
            .cells
            .get_mut(admission_index(admission))
            .and_then(|row| row.get_mut(execution_index(execution)))
            .ok_or_else(|| "selection V4 accounting cell is absent".to_owned())?;
        *slot = slot
            .checked_add(1)
            .ok_or_else(|| "selection V4 accounting cell overflowed u64".to_owned())?;
        Ok(())
    }

    fn validate_against(
        self,
        snapshot: &SelectionAuthoritySnapshotV4,
    ) -> Result<(), SelectionV4Refusal> {
        if self.total()? != snapshot.population.row_count {
            return Err("selection V4 matrix does not account for every population row".to_owned());
        }
        if self.cells != snapshot.execution.admission_execution_counts {
            return Err(
                "selection V4 derived matrix differs from the exact Execution V2 completion matrix"
                    .to_owned(),
            );
        }
        for index in 0..4 {
            if self.admission_marginal(index)? != snapshot.admission.counts.by_index(index) {
                return Err(format!(
                    "selection V4 admission matrix marginal {index} differs from its completion"
                ));
            }
        }
        for index in 0..2 {
            if self.execution_marginal(index)? != snapshot.execution.counts.by_index(index) {
                return Err(format!(
                    "selection V4 execution matrix marginal {index} differs from its completion"
                ));
            }
        }
        Ok(())
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), SelectionV4Refusal> {
        for value in self.cells.into_iter().flatten() {
            encoder.u64(value)?;
        }
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, SelectionV4Refusal> {
        let mut cells = [[0_u64; 2]; 4];
        for row in &mut cells {
            for cell in row {
                *cell = decoder.u64()?;
            }
        }
        Ok(Self { cells })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PassSummaryV4 {
    accounting: SelectionAccountingV4,
    joined_order_digest: [u8; 32],
}

struct PassAccumulatorV4 {
    accounting: SelectionAccountingV4,
    hasher: brutex_core::blake3::Hasher,
}

impl PassAccumulatorV4 {
    fn new(family: InstrumentFamilyV1) -> Self {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(JOINED_ORDER_DOMAIN_V4);
        hasher.update(&[family_byte(family)]);
        Self {
            accounting: SelectionAccountingV4::default(),
            hasher,
        }
    }

    fn observe(&mut self, joined: &SelectionJoinedRowV4) -> Result<(), SelectionV4Refusal> {
        self.accounting
            .charge(joined.admission.status, joined.execution.status)?;
        self.hasher
            .update(&joined.admission.population_payload_digest);
        self.hasher.update(&joined.admission.decision_digest);
        self.hasher
            .update(&[admission_byte(joined.admission.status)]);
        self.hasher.update(&joined.execution.disposition_digest);
        self.hasher
            .update(&[execution_byte(joined.execution.status)]);
        Ok(())
    }

    fn finish(self) -> PassSummaryV4 {
        PassSummaryV4 {
            accounting: self.accounting,
            joined_order_digest: self.hasher.finalize(),
        }
    }
}

/// One exact Population V4 plus admission and Execution V2 authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationReferenceV4 {
    family: InstrumentFamilyV1,
    population_id: [u8; 32],
    row_count: u64,
    ordered_row_digest: [u8; 32],
    population_v4_completion_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
    execution_v2_completion_digest: [u8; 32],
    joined_order_digest: [u8; 32],
    accounting: SelectionAccountingV4,
}

impl PopulationReferenceV4 {
    fn from_snapshot(
        snapshot: &SelectionAuthoritySnapshotV4,
        summary: PassSummaryV4,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionV4Refusal> {
        snapshot.validate(expected_family)?;
        summary.accounting.validate_against(snapshot)?;
        require_digest(
            "joined authority-order digest",
            &summary.joined_order_digest,
        )?;
        let reference = Self {
            family: snapshot.population.family,
            population_id: snapshot.population.population_id,
            row_count: snapshot.population.row_count,
            ordered_row_digest: snapshot.population.ordered_row_digest,
            population_v4_completion_digest: snapshot.population.completion_digest,
            admission_completion_digest: snapshot.admission.completion_digest,
            execution_v2_completion_digest: snapshot.execution.completion_digest,
            joined_order_digest: summary.joined_order_digest,
            accounting: summary.accounting,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }

    fn validate(&self, expected_family: InstrumentFamilyV1) -> Result<(), SelectionV4Refusal> {
        if self.family != expected_family {
            return Err(format!(
                "selection V4 population order is noncanonical: expected {expected_family:?}, found {:?}",
                self.family
            ));
        }
        for (name, digest) in [
            ("population identity", self.population_id),
            ("ordered-row digest", self.ordered_row_digest),
            (
                "Population V4 completion digest",
                self.population_v4_completion_digest,
            ),
            (
                "admission completion digest",
                self.admission_completion_digest,
            ),
            (
                "Execution V2 completion digest",
                self.execution_v2_completion_digest,
            ),
            ("joined authority-order digest", self.joined_order_digest),
        ] {
            require_digest(name, &digest)?;
        }
        if self.accounting.total()? != self.row_count {
            return Err(
                "selection V4 population accounting does not cover its exact row count".to_owned(),
            );
        }
        Ok(())
    }

    /// Canonical instrument family.
    #[must_use]
    pub const fn family(&self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Exact Population V4 identity.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Complete population row count.
    #[must_use]
    pub const fn row_count(&self) -> u64 {
        self.row_count
    }

    /// Digest of canonical population rows in source order.
    #[must_use]
    pub const fn ordered_row_digest(&self) -> [u8; 32] {
        self.ordered_row_digest
    }

    /// Digest of the exact Population V4 completion.
    #[must_use]
    pub const fn population_v4_completion_digest(&self) -> [u8; 32] {
        self.population_v4_completion_digest
    }

    /// Digest of the exact admission completion.
    #[must_use]
    pub const fn admission_completion_digest(&self) -> [u8; 32] {
        self.admission_completion_digest
    }

    /// Digest of the exact Execution V2 completion.
    #[must_use]
    pub const fn execution_v2_completion_digest(&self) -> [u8; 32] {
        self.execution_v2_completion_digest
    }

    /// Digest of every row's population/admission/execution join in order.
    #[must_use]
    pub const fn joined_order_digest(&self) -> [u8; 32] {
        self.joined_order_digest
    }

    /// Exhaustive admission/execution cross-product counts.
    #[must_use]
    pub const fn accounting(&self) -> SelectionAccountingV4 {
        self.accounting
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), SelectionV4Refusal> {
        encoder.u8(family_byte(self.family))?;
        encoder.zeros(7)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        encoder.bytes(&self.population_v4_completion_digest)?;
        encoder.bytes(&self.admission_completion_digest)?;
        encoder.bytes(&self.execution_v2_completion_digest)?;
        encoder.bytes(&self.joined_order_digest)?;
        self.accounting.encode(encoder)
    }

    fn decode(
        decoder: &mut Decoder<'_>,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionV4Refusal> {
        let family = decode_family(decoder.u8()?)?;
        decoder.zeros(7, "population-reference reserve")?;
        let reference = Self {
            family,
            population_id: decoder.array_32()?,
            row_count: decoder.u64()?,
            ordered_row_digest: decoder.array_32()?,
            population_v4_completion_digest: decoder.array_32()?,
            admission_completion_digest: decoder.array_32()?,
            execution_v2_completion_digest: decoder.array_32()?,
            joined_order_digest: decoder.array_32()?,
            accounting: SelectionAccountingV4::decode(decoder)?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }
}

/// Durable global Top-25 derived from the three exact authorities.
///
/// Raw authority views are crate-internal; external callers cannot substitute
/// an implementation for the durable three-ledger adapter.
///
/// ```compile_fail
/// use cli::selection_v4::SelectionAuthorityViewV4;
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionReceiptV4 {
    selection_id: [u8; 32],
    rung_seconds: u32,
    populations: [PopulationReferenceV4; 2],
    proof: PopulationProof,
    ranking_policy_digest: [u8; 32],
    cohort: SharedCohortIdentityV2,
    selected: Vec<SelectedEntryV1>,
}

impl SelectionReceiptV4 {
    /// Recomputes the exact global Top-25 from two crate-owned authority views.
    ///
    /// # Errors
    ///
    /// Refuses family/cohort/policy mismatch, incomplete or changed snapshots,
    /// missing/duplicate/misaligned rows, authority-count mismatch, two-pass
    /// disagreement, ambiguous selected resolution or invalid fixed fields.
    pub(crate) fn from_authorities(
        nifty: &mut dyn SelectionAuthorityViewV4,
        bank_nifty: &mut dyn SelectionAuthorityViewV4,
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionV4Refusal> {
        let nifty_snapshot = nifty.snapshot().clone();
        let bank_snapshot = bank_nifty.snapshot().clone();
        let ranking_policy_digest = policy.digest();
        validate_global_authority_pair(&nifty_snapshot, &bank_snapshot, ranking_policy_digest)?;

        let mut extrema = Extrema::default();
        let mut first_pass = PopulationPass::new();
        let nifty_first = visit_one_authority(
            nifty,
            &nifty_snapshot,
            InstrumentFamilyV1::Nifty,
            &mut |_, candidate| {
                extrema.observe(&candidate);
                first_pass.observe(&candidate);
                Ok(())
            },
        )?;
        let bank_first = visit_one_authority(
            bank_nifty,
            &bank_snapshot,
            InstrumentFamilyV1::BankNifty,
            &mut |_, candidate| {
                extrema.observe(&candidate);
                first_pass.observe(&candidate);
                Ok(())
            },
        )?;
        let proof = first_pass.finish();
        validate_combined_proof(proof, nifty_first, bank_first)?;

        let references = [
            PopulationReferenceV4::from_snapshot(
                &nifty_snapshot,
                nifty_first,
                InstrumentFamilyV1::Nifty,
            )?,
            PopulationReferenceV4::from_snapshot(
                &bank_snapshot,
                bank_first,
                InstrumentFamilyV1::BankNifty,
            )?,
        ];

        let mut keeper = VerifiedKeeper::new(MAX_TOP, policy, extrema, proof)
            .map_err(|why| format!("global Selection V4 Top-25 could not start: {why:?}"))?;
        let nifty_second = visit_one_authority(
            nifty,
            &nifty_snapshot,
            InstrumentFamilyV1::Nifty,
            &mut |_, candidate| {
                keeper.offer(candidate).map_err(|why| {
                    format!("global Selection V4 Top-25 candidate was refused: {why:?}")
                })
            },
        )?;
        let bank_second = visit_one_authority(
            bank_nifty,
            &bank_snapshot,
            InstrumentFamilyV1::BankNifty,
            &mut |_, candidate| {
                keeper.offer(candidate).map_err(|why| {
                    format!("global Selection V4 Top-25 candidate was refused: {why:?}")
                })
            },
        )?;
        if [nifty_second, bank_second] != [nifty_first, bank_first] {
            return Err(
                "Selection V4 second-pass authority accounting differs from its first pass"
                    .to_owned(),
            );
        }
        let selection = keeper.finish().map_err(|why| {
            format!("global Selection V4 Top-25 did not reproduce its first pass: {why:?}")
        })?;
        validate_selection_reconciliation(&selection, proof)?;

        let (selected, third) = resolve_selected(
            nifty,
            bank_nifty,
            &nifty_snapshot,
            &bank_snapshot,
            &references,
            &selection,
        )?;
        if third != [nifty_first, bank_first] {
            return Err(
                "Selection V4 selected-row pass differs from its sealed authority pass".to_owned(),
            );
        }

        let mut receipt = Self {
            selection_id: [0; 32],
            rung_seconds: nifty_snapshot.population.rung_seconds,
            populations: references,
            proof,
            ranking_policy_digest,
            cohort: nifty_snapshot.population.cohort,
            selected,
        };
        receipt.selection_id = receipt.derived_id()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }

    /// Rebuilds and byte-compares this receipt against crate-owned authorities.
    ///
    /// # Errors
    ///
    /// Every construction refusal, or any canonical field differing from the
    /// complete rebuilt receipt.
    pub(crate) fn verify_against_authorities(
        &self,
        nifty: &mut dyn SelectionAuthorityViewV4,
        bank_nifty: &mut dyn SelectionAuthorityViewV4,
        policy: RankingPolicyV1,
    ) -> Result<(), SelectionV4Refusal> {
        let rebuilt = Self::from_authorities(nifty, bank_nifty, policy)?;
        if rebuilt != *self {
            return Err(
                "Selection V4 differs from a full replay of its three authorities".to_owned(),
            );
        }
        Ok(())
    }

    /// Content-derived V4 selection identity.
    #[must_use]
    pub const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    /// Shared signal timeframe.
    #[must_use]
    pub const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    /// Canonical `[NIFTY, BANKNIFTY]` triple-authority references.
    #[must_use]
    pub const fn populations(&self) -> &[PopulationReferenceV4; 2] {
        &self.populations
    }

    /// Exact combined candidate proof.
    #[must_use]
    pub const fn population_proof(&self) -> PopulationProof {
        self.proof
    }

    /// Canonical final-ranking policy identity.
    #[must_use]
    pub const fn ranking_policy_digest(&self) -> [u8; 32] {
        self.ranking_policy_digest
    }

    /// Full shared calendar/exit-policy cohort identity.
    #[must_use]
    pub const fn cohort_identity(&self) -> SharedCohortIdentityV2 {
        self.cohort
    }

    /// Digest used for exact cohort discovery.
    #[must_use]
    pub fn cohort_digest(&self) -> [u8; 32] {
        self.cohort.digest()
    }

    /// Strongest-first Top-25, or every final-eligible row when fewer exist.
    #[must_use]
    pub fn top_twenty_five(&self) -> &[SelectedEntryV1] {
        &self.selected
    }

    /// First ten entries of the exact persisted Top-25 ordering.
    #[must_use]
    pub fn top_ten(&self) -> &[SelectedEntryV1] {
        let end = self.selected.len().min(10);
        self.selected.get(..end).unwrap_or(&[])
    }

    fn discovery_key(&self) -> ([u8; 32], u32) {
        (self.cohort.digest(), self.rung_seconds)
    }

    fn validate_semantics(&self) -> Result<(), SelectionV4Refusal> {
        require_digest("selection identity", &self.selection_id)?;
        require_digest("ranking-policy digest", &self.ranking_policy_digest)?;
        require_digest("ordered candidate-proof digest", &self.proof.ordered_digest)?;
        if !CANONICAL_RUNGS.contains(&self.rung_seconds) {
            return Err(format!(
                "Selection V4 timeframe {} seconds is outside the canonical rungs",
                self.rung_seconds
            ));
        }
        self.cohort.validate()?;
        if self.cohort.ranking_policy_digest() != self.ranking_policy_digest {
            return Err(
                "Selection V4 ranking-policy digest differs from its cohort identity".to_owned(),
            );
        }
        let [nifty, bank_nifty] = &self.populations;
        nifty.validate(InstrumentFamilyV1::Nifty)?;
        bank_nifty.validate(InstrumentFamilyV1::BankNifty)?;
        if nifty.population_id == bank_nifty.population_id {
            return Err("both Selection V4 families reference the same population".to_owned());
        }
        let considered = checked_add(
            nifty.row_count,
            bank_nifty.row_count,
            "combined population row counts",
        )?;
        let eligible = checked_add(
            nifty.accounting.eligible(),
            bank_nifty.accounting.eligible(),
            "combined eligible row counts",
        )?;
        if self.proof.considered != considered || self.proof.admitted != eligible {
            return Err(
                "Selection V4 proof does not reconcile to its population/accounting references"
                    .to_owned(),
            );
        }
        if checked_add(
            self.proof.admitted,
            self.proof.refused,
            "proof eligibility counts",
        )? != considered
        {
            return Err(
                "Selection V4 admitted + ineligible counts do not equal considered".to_owned(),
            );
        }
        if self.proof.unmeasured > considered {
            return Err("Selection V4 undefined-metric count exceeds considered".to_owned());
        }
        let expected_selected = usize::try_from(eligible).unwrap_or(usize::MAX).min(MAX_TOP);
        if self.selected.len() != expected_selected {
            return Err(format!(
                "Selection V4 stores {} winner(s), but eligible={eligible} requires {expected_selected}",
                self.selected.len()
            ));
        }
        for entry in &self.selected {
            validate_selected_entry(*entry, nifty, bank_nifty)?;
        }
        for (index, left) in self.selected.iter().enumerate() {
            for right in self.selected.iter().skip(index.saturating_add(1)) {
                if left.population_id() == right.population_id()
                    && left.row_sequence() == right.row_sequence()
                {
                    return Err("Selection V4 repeats one population row".to_owned());
                }
                if left.strategy_digest() == right.strategy_digest() {
                    return Err("Selection V4 repeats one semantic strategy digest".to_owned());
                }
            }
        }
        if self.selected.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(stronger, weaker)| stronger.score() < weaker.score())
        }) {
            return Err("Selection V4 scores are not strongest-first".to_owned());
        }
        if self.derived_id()? != self.selection_id {
            return Err(
                "Selection V4 identity does not match its canonical authority content".to_owned(),
            );
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], SelectionV4Refusal> {
        let payload = self.payload_with_id([0; 32])?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(ID_DOMAIN_V4);
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "Selection V4 identity payload is absent".to_owned())?,
        );
        Ok(hasher.finalize())
    }

    fn payload_with_id(
        &self,
        selection_id: [u8; 32],
    ) -> Result<[u8; PAYLOAD_BYTES_V4], SelectionV4Refusal> {
        let mut payload = [0_u8; PAYLOAD_BYTES_V4];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&selection_id)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u32(REQUESTED_TOP_V4)?;
        let [nifty, bank_nifty] = &self.populations;
        nifty.encode(&mut encoder)?;
        bank_nifty.encode(&mut encoder)?;
        for count in [
            self.proof.considered,
            self.proof.admitted,
            self.proof.refused,
            self.proof.unmeasured,
        ] {
            encoder.u64(count)?;
        }
        encoder.bytes(&self.proof.ordered_digest)?;
        encoder.bytes(&self.ranking_policy_digest)?;
        encoder.bytes(&self.cohort.canonical_bytes())?;
        encoder.u32(
            u32::try_from(self.selected.len())
                .map_err(|_| "Selection V4 winner count does not fit u32".to_owned())?,
        )?;
        encoder.zeros(4)?;
        for entry in &self.selected {
            encoder.bytes(&entry.canonical_bytes()?)?;
        }
        encoder.zeros(
            MAX_TOP
                .saturating_sub(self.selected.len())
                .saturating_mul(SELECTED_ENTRY_CANONICAL_LEN_V1),
        )?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(payload)
    }

    fn to_bytes(&self) -> Result<[u8; SELECTION_V4_STRIDE_BYTES], SelectionV4Refusal> {
        self.validate_semantics()?;
        let payload = self.payload_with_id(self.selection_id)?;
        let mut raw = [0_u8; SELECTION_V4_STRIDE_BYTES];
        raw.get_mut(..PAYLOAD_BYTES_V4)
            .ok_or_else(|| "Selection V4 payload slot is absent".to_owned())?
            .copy_from_slice(&payload);
        raw.get_mut(PAYLOAD_BYTES_V4..)
            .ok_or_else(|| "Selection V4 seal slot is absent".to_owned())?
            .copy_from_slice(&brutex_core::blake3::hash(&payload));
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; SELECTION_V4_STRIDE_BYTES]) -> Result<Self, SelectionV4Refusal> {
        let payload = raw
            .get(..PAYLOAD_BYTES_V4)
            .ok_or_else(|| "Selection V4 payload is absent".to_owned())?;
        let stored_seal = raw
            .get(PAYLOAD_BYTES_V4..)
            .ok_or_else(|| "Selection V4 seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("Selection V4 receipt failed its complete BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let selection_id = decoder.array_32()?;
        let rung_seconds = decoder.u32()?;
        let requested = decoder.u32()?;
        if requested != REQUESTED_TOP_V4 {
            return Err(format!(
                "Selection V4 requests {requested}; version four requires exactly {REQUESTED_TOP_V4}"
            ));
        }
        let populations = [
            PopulationReferenceV4::decode(&mut decoder, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV4::decode(&mut decoder, InstrumentFamilyV1::BankNifty)?,
        ];
        let proof = PopulationProof {
            considered: decoder.u64()?,
            admitted: decoder.u64()?,
            refused: decoder.u64()?,
            unmeasured: decoder.u64()?,
            ordered_digest: decoder.array_32()?,
        };
        let ranking_policy_digest = decoder.array_32()?;
        let cohort = SharedCohortIdentityV2::from_canonical_bytes(
            decoder.take(SHARED_COHORT_CANONICAL_LEN_V2)?,
        )?;
        let selected_count = usize::try_from(decoder.u32()?)
            .map_err(|_| "Selection V4 winner count does not fit this machine".to_owned())?;
        if selected_count > MAX_TOP {
            return Err(format!(
                "Selection V4 stores {selected_count} rows above fixed maximum {MAX_TOP}"
            ));
        }
        decoder.zeros(4, "selected-count reserve")?;
        let mut selected = Vec::new();
        selected.try_reserve_exact(selected_count).map_err(|why| {
            format!("Selection V4 could not reserve {selected_count} bounded winners: {why}")
        })?;
        for _ in 0..selected_count {
            selected.push(SelectedEntryV1::from_canonical_bytes(
                decoder.take(SELECTED_ENTRY_CANONICAL_LEN_V1)?,
            )?);
        }
        decoder.zeros(
            MAX_TOP
                .saturating_sub(selected_count)
                .saturating_mul(SELECTED_ENTRY_CANONICAL_LEN_V1),
            "unused selected-entry slots",
        )?;
        decoder.zeros(8, "trailing reserve")?;
        decoder.finish()?;
        let receipt = Self {
            selection_id,
            rung_seconds,
            populations,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

fn validate_global_authority_pair(
    nifty: &SelectionAuthoritySnapshotV4,
    bank_nifty: &SelectionAuthoritySnapshotV4,
    ranking_policy_digest: [u8; 32],
) -> Result<(), SelectionV4Refusal> {
    nifty.validate(InstrumentFamilyV1::Nifty)?;
    bank_nifty.validate(InstrumentFamilyV1::BankNifty)?;
    if nifty.population.population_id == bank_nifty.population.population_id {
        return Err(
            "NIFTY and BANKNIFTY Selection V4 authorities cannot share one population identity"
                .to_owned(),
        );
    }
    if nifty.population.rung_seconds != bank_nifty.population.rung_seconds {
        return Err(format!(
            "global Selection V4 requires one timeframe, but NIFTY is {}s and BANKNIFTY is {}s",
            nifty.population.rung_seconds, bank_nifty.population.rung_seconds
        ));
    }
    if nifty.population.cohort != bank_nifty.population.cohort {
        return Err(
            "NIFTY and BANKNIFTY Selection V4 authorities have different shared cohorts".to_owned(),
        );
    }
    if nifty.population.ranking_policy_digest != ranking_policy_digest
        || bank_nifty.population.ranking_policy_digest != ranking_policy_digest
    {
        return Err(
            "Selection V4 Population V4 ranking policy differs from the requested policy"
                .to_owned(),
        );
    }
    Ok(())
}

fn visit_one_authority<F>(
    authority: &mut dyn SelectionAuthorityViewV4,
    expected_snapshot: &SelectionAuthoritySnapshotV4,
    expected_family: InstrumentFamilyV1,
    visit: &mut F,
) -> Result<PassSummaryV4, SelectionV4Refusal>
where
    F: FnMut(&SelectionJoinedRowV4, Candidate) -> Result<(), SelectionV4Refusal>,
{
    expected_snapshot.validate(expected_family)?;
    if authority.snapshot() != expected_snapshot {
        return Err(format!(
            "{expected_family:?} Selection V4 authority changed before its pass began"
        ));
    }
    let mut accumulator = PassAccumulatorV4::new(expected_family);
    let mut offset = 0_u64;
    while offset < expected_snapshot.population.row_count {
        let remaining = expected_snapshot
            .population
            .row_count
            .checked_sub(offset)
            .ok_or_else(|| "Selection V4 page remaining count underflowed".to_owned())?;
        let requested = remaining.min(MAX_PAGE_ROWS_V1);
        let page = authority.page(offset, requested)?;
        validate_page(&page, expected_snapshot, offset, requested)?;
        for joined in page.rows {
            validate_joined_row(&joined, expected_snapshot, offset)?;
            accumulator.observe(&joined)?;
            let candidate = candidate_from_joined(&joined);
            visit(&joined, candidate)?;
            offset = offset
                .checked_add(1)
                .ok_or_else(|| "Selection V4 joined-row offset overflowed u64".to_owned())?;
        }
    }
    if authority.snapshot() != expected_snapshot {
        return Err(format!(
            "{expected_family:?} Selection V4 authority changed before its pass completed"
        ));
    }
    let summary = accumulator.finish();
    summary.accounting.validate_against(expected_snapshot)?;
    require_digest(
        "joined authority-order digest",
        &summary.joined_order_digest,
    )?;
    Ok(summary)
}

fn validate_page(
    page: &SelectionAuthorityPageV4,
    snapshot: &SelectionAuthoritySnapshotV4,
    offset: u64,
    requested: u64,
) -> Result<(), SelectionV4Refusal> {
    if requested == 0 || requested > MAX_PAGE_ROWS_V1 {
        return Err(format!(
            "Selection V4 requested noncanonical page size {requested}"
        ));
    }
    let expected_len = usize::try_from(requested)
        .map_err(|_| "Selection V4 page size does not fit this machine".to_owned())?;
    if page.total != snapshot.population.row_count
        || page.offset != offset
        || page.rows.len() != expected_len
    {
        return Err(format!(
            "population {} returned a noncontiguous Selection V4 page at offset {offset}",
            hex(&snapshot.population.population_id)
        ));
    }
    if page.population_v4_completion_digest != snapshot.population.completion_digest
        || page.admission_completion_digest != snapshot.admission.completion_digest
        || page.execution_completion_digest != snapshot.execution.completion_digest
    {
        return Err(format!(
            "population {} page was read under different completion authorities",
            hex(&snapshot.population.population_id)
        ));
    }
    Ok(())
}

fn validate_joined_row(
    joined: &SelectionJoinedRowV4,
    snapshot: &SelectionAuthoritySnapshotV4,
    expected_sequence: u64,
) -> Result<(), SelectionV4Refusal> {
    let row = joined.population;
    if row.population_id != snapshot.population.population_id
        || row.sequence != expected_sequence
        || row.instrument_family != snapshot.population.family
        || row.rung_seconds != snapshot.population.rung_seconds
    {
        return Err(format!(
            "population {} row at offset {expected_sequence} disagrees with its Selection V4 snapshot",
            hex(&snapshot.population.population_id)
        ));
    }
    let payload_digest = row.payload_digest()?;
    for (name, population_id, sequence, strategy_digest, bound_payload) in [
        (
            "admission",
            joined.admission.population_id,
            joined.admission.sequence,
            joined.admission.strategy_digest,
            joined.admission.population_payload_digest,
        ),
        (
            "execution",
            joined.execution.population_id,
            joined.execution.sequence,
            joined.execution.strategy_digest,
            joined.execution.population_payload_digest,
        ),
    ] {
        if population_id != row.population_id
            || sequence != row.sequence
            || strategy_digest != row.strategy_digest
            || bound_payload != payload_digest
        {
            return Err(format!(
                "Selection V4 {name} row at sequence {expected_sequence} does not bind the exact population payload"
            ));
        }
    }
    require_digest(
        "admission decision digest",
        &joined.admission.decision_digest,
    )?;
    require_digest(
        "Execution V2 disposition digest",
        &joined.execution.disposition_digest,
    )?;
    if joined.execution.population_v4_completion_digest != snapshot.population.completion_digest
        || joined.execution.admission_completion_digest != snapshot.admission.completion_digest
        || joined.execution.admission_status != joined.admission.status
    {
        return Err(format!(
            "Selection V4 execution row at sequence {expected_sequence} binds different upstream completions"
        ));
    }
    Ok(())
}

fn candidate_from_joined(joined: &SelectionJoinedRowV4) -> Candidate {
    let row = joined.population;
    Candidate {
        strategy_digest: StrategyDigest::new(row.strategy_digest),
        mask_words: row.mask_words,
        direction: match row.direction {
            TradeDirectionV1::Long => Direction::Long,
            TradeDirectionV1::Short => Direction::Short,
        },
        admitted: matches!(joined.admission.status, AdmissionStatusV1::Admitted)
            && matches!(
                joined.execution.status,
                ExecutionSelectionStatusV2::Authorized
            ),
        metrics: metrics_from_row(row.metrics),
    }
}

const fn metrics_from_row(metrics: TopMetricsV1) -> Metrics {
    Metrics {
        drawdown: metrics.drawdown,
        worst_loss: metrics.worst_loss,
        losing_rate_ppm: metrics.losing_rate_ppm,
        losing_trades: metrics.losing_trades,
        loss_ratio_ppm: metrics.loss_ratio_ppm,
        pessimistic_profit: metrics.pessimistic_profit,
        winning_trades: metrics.winning_trades,
        win_rate_ppm: metrics.win_rate_ppm,
        reward_to_risk_ppm: metrics.reward_to_risk_ppm,
        average_win: metrics.average_win,
        average_loss: metrics.average_loss,
        assurance_ppm: metrics.assurance_ppm,
    }
}

fn validate_combined_proof(
    proof: PopulationProof,
    nifty: PassSummaryV4,
    bank_nifty: PassSummaryV4,
) -> Result<(), SelectionV4Refusal> {
    let considered = checked_add(
        nifty.accounting.total()?,
        bank_nifty.accounting.total()?,
        "combined authority-accounted rows",
    )?;
    let admitted = checked_add(
        nifty.accounting.eligible(),
        bank_nifty.accounting.eligible(),
        "combined final-eligible rows",
    )?;
    let refused = considered
        .checked_sub(admitted)
        .ok_or_else(|| "Selection V4 eligible count exceeds considered".to_owned())?;
    if proof.considered != considered
        || proof.admitted != admitted
        || proof.refused != refused
        || proof.unmeasured > considered
    {
        return Err(format!(
            "Selection V4 proof {}/{}/{}/{} does not reconcile to {considered}/{admitted}/{refused}/<=considered",
            proof.considered, proof.admitted, proof.refused, proof.unmeasured
        ));
    }
    require_digest("combined ordered candidate digest", &proof.ordered_digest)
}

fn validate_selection_reconciliation(
    selection: &Selection,
    proof: PopulationProof,
) -> Result<(), SelectionV4Refusal> {
    if selection.requested != MAX_TOP {
        return Err(format!(
            "ranking kernel returned requested={}, not authoritative {MAX_TOP}",
            selection.requested
        ));
    }
    if selection.considered != proof.considered
        || selection.admitted != proof.admitted
        || selection.refused != proof.refused
        || selection.unmeasured != proof.unmeasured
    {
        return Err("ranking selection counts do not reproduce the Selection V4 proof".to_owned());
    }
    let expected = usize::try_from(proof.admitted)
        .unwrap_or(usize::MAX)
        .min(MAX_TOP);
    if selection.rows.len() != expected {
        return Err(format!(
            "ranking kernel returned {} rows, but eligible={} requires {expected}",
            selection.rows.len(),
            proof.admitted
        ));
    }
    if selection.rows.windows(2).any(|pair| {
        pair.first()
            .zip(pair.get(1))
            .is_some_and(|(stronger, weaker)| stronger < weaker)
    }) {
        return Err("ranking kernel output is not strongest-first".to_owned());
    }
    Ok(())
}

fn resolve_selected(
    nifty: &mut dyn SelectionAuthorityViewV4,
    bank_nifty: &mut dyn SelectionAuthorityViewV4,
    nifty_snapshot: &SelectionAuthoritySnapshotV4,
    bank_snapshot: &SelectionAuthoritySnapshotV4,
    references: &[PopulationReferenceV4; 2],
    selection: &Selection,
) -> Result<(Vec<SelectedEntryV1>, [PassSummaryV4; 2]), SelectionV4Refusal> {
    let [nifty_reference, bank_reference] = references;
    let mut resolved = vec![None; selection.rows.len()];
    let nifty_summary = visit_one_authority(
        nifty,
        nifty_snapshot,
        InstrumentFamilyV1::Nifty,
        &mut |joined, candidate| {
            resolve_candidate(
                &mut resolved,
                selection,
                nifty_reference,
                joined.population.sequence,
                joined.population.strategy_digest,
                candidate,
            )
        },
    )?;
    let bank_summary = visit_one_authority(
        bank_nifty,
        bank_snapshot,
        InstrumentFamilyV1::BankNifty,
        &mut |joined, candidate| {
            resolve_candidate(
                &mut resolved,
                selection,
                bank_reference,
                joined.population.sequence,
                joined.population.strategy_digest,
                candidate,
            )
        },
    )?;
    let selected = resolved
        .into_iter()
        .enumerate()
        .map(|(rank, entry)| {
            entry.ok_or_else(|| {
                format!(
                    "global Selection V4 rank {} has no triple-authoritative source row",
                    rank + 1
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((selected, [nifty_summary, bank_summary]))
}

fn resolve_candidate(
    resolved: &mut [Option<SelectedEntryV1>],
    selection: &Selection,
    reference: &PopulationReferenceV4,
    row_sequence: u64,
    row_strategy_digest: [u8; 32],
    candidate: Candidate,
) -> Result<(), SelectionV4Refusal> {
    for (index, ranked) in selection.rows.iter().enumerate() {
        if candidate.strategy_digest != ranked.candidate.strategy_digest {
            continue;
        }
        if candidate != ranked.candidate || candidate.strategy_digest.bytes() != row_strategy_digest
        {
            return Err(format!(
                "strategy digest {} aliases different Selection V4 candidate bytes",
                hex(&candidate.strategy_digest.bytes())
            ));
        }
        let slot = resolved
            .get_mut(index)
            .ok_or_else(|| "bounded Selection V4 winner slot is absent".to_owned())?;
        if slot.is_some() {
            return Err(format!(
                "strategy digest {} occurs more than once in the joined authorities",
                hex(&candidate.strategy_digest.bytes())
            ));
        }
        *slot = Some(SelectedEntryV1::new(
            reference.family,
            reference.population_id,
            row_sequence,
            row_strategy_digest,
            ranked.score,
        ));
    }
    Ok(())
}

fn validate_selected_entry(
    entry: SelectedEntryV1,
    nifty: &PopulationReferenceV4,
    bank_nifty: &PopulationReferenceV4,
) -> Result<(), SelectionV4Refusal> {
    require_digest("selected population identity", &entry.population_id())?;
    require_digest("selected strategy digest", &entry.strategy_digest())?;
    if entry.score() > SCORE_SCALE {
        return Err(format!(
            "selected score {} exceeds fixed-point domain 0..={SCORE_SCALE}",
            entry.score()
        ));
    }
    let source = match entry.family() {
        InstrumentFamilyV1::Nifty => nifty,
        InstrumentFamilyV1::BankNifty => bank_nifty,
    };
    if entry.population_id() != source.population_id {
        return Err(format!(
            "selected {:?} row names a population other than its canonical source",
            entry.family()
        ));
    }
    if entry.row_sequence() >= source.row_count {
        return Err(format!(
            "selected {:?} sequence {} is outside its {}-row population",
            entry.family(),
            entry.row_sequence(),
            source.row_count
        ));
    }
    Ok(())
}

/// Append-only fixed-stride Selection V4 ledger.
#[derive(Debug)]
pub struct SelectionLedgerV4 {
    file: File,
    path: PathBuf,
    receipts: HashMap<[u8; 32], SelectionReceiptV4>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
    scanned: u64,
    generation: FileGeneration,
    writable: bool,
    max_receipts: usize,
}

impl SelectionLedgerV4 {
    /// Version-four ledger path. Earlier selection files are never consulted.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("global-selections-v4.bin")
    }

    /// Opens or creates the V4 ledger under an explicit receipt bound.
    ///
    /// # Errors
    ///
    /// Refuses a zero bound, I/O/locking failure, non-V4 header, ragged/torn
    /// record, malformed receipt, duplicate identity or an oversized file.
    pub fn open(root: &Path, max_receipts: usize) -> Result<Self, SelectionV4Refusal> {
        validate_receipt_limit(max_receipts)?;
        ensure_results_directory(root)?;
        let path = Self::path(root);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        Self::open_file(file, path, true, max_receipts)
    }

    /// Opens an existing V4 ledger read-only under an explicit receipt bound.
    ///
    /// No V1/V2/V3 path or migration fallback is attempted.
    ///
    /// # Errors
    ///
    /// Every structural refusal from [`Self::open`], plus absence or an empty
    /// read-only file.
    pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionV4Refusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
        Self::open_file(file, path, false, max_receipts)
    }

    fn open_file(
        mut file: File,
        path: PathBuf,
        writable: bool,
        max_receipts: usize,
    ) -> Result<Self, SelectionV4Refusal> {
        if writable {
            file.lock().map_err(|why| {
                format!("{} could not be exclusively locked: {why}", path.display())
            })?;
        } else {
            file.lock_shared()
                .map_err(|why| format!("{} could not be shared-locked: {why}", path.display()))?;
        }
        let opened = (|| {
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be read: {why}", path.display()))?
                .len();
            if len == 0 {
                if !writable {
                    return Err(format!("{} is empty and read-only", path.display()));
                }
                write_header(&mut file)?;
                file.sync_all().map_err(|why| {
                    format!("{} V4 header could not be synced: {why}", path.display())
                })?;
            }
            let len = file
                .metadata()
                .map_err(|why| format!("{} length could not be reread: {why}", path.display()))?
                .len();
            let indexes = scan_file(&mut file, &path, len, max_receipts)?;
            let generation = validated_generation(&file, &path, len)?;
            Ok((indexes, len, generation))
        })();
        let released = file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after V4 open: {why}",
                path.display()
            )
        });
        match (opened, released) {
            (Ok((indexes, scanned, generation)), Ok(())) => Ok(Self {
                file,
                path,
                receipts: indexes.receipts,
                order: indexes.order,
                latest: indexes.latest,
                scanned,
                generation,
                writable,
                max_receipts,
            }),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Number of indexed V4 receipts.
    #[must_use]
    pub fn selections(&self) -> usize {
        self.order.len()
    }

    /// Selection identities in exact append order.
    #[must_use]
    pub fn selection_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Average-O(1) exact receipt lookup; no worst-case O(1) claim is made.
    #[must_use]
    pub fn receipt(&self, selection_id: &[u8; 32]) -> Option<&SelectionReceiptV4> {
        self.receipts.get(selection_id)
    }

    /// Latest append for one exact cohort digest and signal rung.
    #[must_use]
    pub fn latest_selection(
        &self,
        cohort_digest: [u8; 32],
        rung_seconds: u32,
    ) -> Option<&SelectionReceiptV4> {
        self.latest
            .get(&(cohort_digest, rung_seconds))
            .and_then(|selection_id| self.receipts.get(selection_id))
    }

    /// Appends and syncs one complete fixed-stride V4 receipt.
    ///
    /// A stale or externally extended handle fails closed and must be reopened;
    /// it never absorbs bytes under a previously validated snapshot.
    ///
    /// # Errors
    ///
    /// Refuses read-only use, invalid/duplicate bytes, a stale or replaced file,
    /// the caller's bound, arithmetic failure and every I/O or lock failure.
    pub fn append(&mut self, receipt: &SelectionReceiptV4) -> Result<(), SelectionV4Refusal> {
        if !self.writable {
            return Err("a read-only Selection V4 ledger cannot append".to_owned());
        }
        receipt.validate_semantics()?;
        self.file.lock().map_err(|why| {
            format!(
                "{} could not be locked for V4 append: {why}",
                self.path.display()
            )
        })?;
        let attempted = (|| {
            let observed = file_generation(&self.file, &self.path)?;
            require_generation_unchanged(self.generation, observed, &self.path)?;
            if observed.len != self.scanned {
                return Err(format!(
                    "{} changed from {} to {} bytes; reopen before append",
                    self.path.display(),
                    self.scanned,
                    observed.len
                ));
            }
            self.file
                .seek(SeekFrom::Start(0))
                .map_err(|why| format!("Selection V4 header seek failed: {why}"))?;
            let mut header = [0_u8; HEADER_BYTES];
            self.file
                .read_exact(&mut header)
                .map_err(|why| format!("Selection V4 header recheck failed: {why}"))?;
            validate_header(&header)?;
            if self.receipts.contains_key(&receipt.selection_id) {
                return Err(format!(
                    "Selection V4 {} is already present; duplicate identities are refused",
                    hex(&receipt.selection_id)
                ));
            }
            if self.order.len() >= self.max_receipts {
                return Err(format!(
                    "Selection V4 ledger already contains its caller maximum of {} receipt(s)",
                    self.max_receipts
                ));
            }
            self.receipts
                .try_reserve(1)
                .map_err(|why| format!("Selection V4 index reserve failed: {why}"))?;
            self.order
                .try_reserve(1)
                .map_err(|why| format!("Selection V4 order reserve failed: {why}"))?;
            self.latest
                .try_reserve(1)
                .map_err(|why| format!("Selection V4 latest-index reserve failed: {why}"))?;
            let raw = receipt.to_bytes()?;
            self.file
                .seek(SeekFrom::End(0))
                .map_err(|why| format!("Selection V4 append seek failed: {why}"))?;
            self.file
                .write_all(&raw)
                .map_err(|why| format!("Selection V4 receipt append failed: {why}"))?;
            self.file
                .sync_all()
                .map_err(|why| format!("Selection V4 receipt sync failed: {why}"))?;
            self.scanned = self
                .scanned
                .checked_add(SELECTION_V4_STRIDE)
                .ok_or_else(|| "Selection V4 scanned length overflowed u64".to_owned())?;
            self.generation = validated_generation(&self.file, &self.path, self.scanned)?;
            let selection_id = receipt.selection_id;
            self.latest.insert(receipt.discovery_key(), selection_id);
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
            Ok(())
        })();
        let released = self.file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after V4 append: {why}",
                self.path.display()
            )
        });
        match (attempted, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
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
fn ensure_results_directory(root: &Path) -> Result<(), SelectionV4Refusal> {
    let root_metadata = std::fs::metadata(root).map_err(|why| {
        format!(
            "Selection V4 authority root {} must already exist as a directory: {why}",
            root.display()
        )
    })?;
    if !root_metadata.is_dir() {
        return Err(format!(
            "Selection V4 authority root {} is not a directory",
            root.display()
        ));
    }

    let results = root.join("results");
    match std::fs::create_dir(&results) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::metadata(&results).map_err(|metadata_why| {
                format!(
                    "Selection V4 ledger directory {} could not be inspected after an existing-path result ({why}): {metadata_why}",
                    results.display()
                )
            })?;
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(format!(
                    "Selection V4 ledger directory {} exists but is not a directory",
                    results.display()
                ))
            }
        }
        Err(why) => Err(format!(
            "Selection V4 ledger directory {} could not be created beneath the admitted root: {why}",
            results.display()
        )),
    }
}

#[derive(Debug)]
struct SelectionIndexesV4 {
    receipts: HashMap<[u8; 32], SelectionReceiptV4>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl SelectionIndexesV4 {
    fn with_capacity(capacity: usize) -> Result<Self, SelectionV4Refusal> {
        let mut receipts = HashMap::new();
        receipts
            .try_reserve(capacity)
            .map_err(|why| format!("Selection V4 index reserve failed: {why}"))?;
        let mut order = Vec::new();
        order
            .try_reserve_exact(capacity)
            .map_err(|why| format!("Selection V4 order reserve failed: {why}"))?;
        let mut latest = HashMap::new();
        latest
            .try_reserve(capacity)
            .map_err(|why| format!("Selection V4 latest-index reserve failed: {why}"))?;
        Ok(Self {
            receipts,
            order,
            latest,
        })
    }

    fn insert(
        &mut self,
        receipt: SelectionReceiptV4,
        location: &str,
    ) -> Result<(), SelectionV4Refusal> {
        let selection_id = receipt.selection_id;
        if self.receipts.contains_key(&selection_id) {
            return Err(format!(
                "{location} repeats Selection V4 identity {}",
                hex(&selection_id)
            ));
        }
        self.latest.insert(receipt.discovery_key(), selection_id);
        self.receipts.insert(selection_id, receipt);
        self.order.push(selection_id);
        Ok(())
    }
}

fn write_header(file: &mut File) -> Result<(), SelectionV4Refusal> {
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&MAGIC_V4)?;
    encoder.u32(VERSION_V4)?;
    encoder.u32(HEADER_BYTES_U32)?;
    encoder.u32(STRIDE_BYTES_U32_V4)?;
    encoder.u32(REQUESTED_TOP_V4)?;
    encoder.u64(SCORE_SCALE)?;
    encoder.zeros(8)?;
    encoder.finish()?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("Selection V4 header seek failed: {why}"))?;
    file.write_all(&header)
        .map_err(|why| format!("Selection V4 header write failed: {why}"))
}

fn scan_file(
    file: &mut File,
    path: &Path,
    len: u64,
    max_receipts: usize,
) -> Result<SelectionIndexesV4, SelectionV4Refusal> {
    validate_file_length(len)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} V4 header seek failed: {why}", path.display()))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("{} V4 header read failed: {why}", path.display()))?;
    validate_header(&header)?;
    let count = receipt_capacity(len, max_receipts)?;
    let mut indexes = SelectionIndexesV4::with_capacity(count)?;
    for index in 0..count {
        let index_u64 = u64::try_from(index)
            .map_err(|_| "Selection V4 scan index does not fit u64".to_owned())?;
        let location = index_u64
            .checked_mul(SELECTION_V4_STRIDE)
            .and_then(|offset| HEADER_BYTES_U64.checked_add(offset))
            .ok_or_else(|| "Selection V4 scan offset overflowed u64".to_owned())?;
        let mut raw = [0_u8; SELECTION_V4_STRIDE_BYTES];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} Selection V4 record at byte {location} could not be read: {why}",
                path.display()
            )
        })?;
        let receipt = SelectionReceiptV4::from_bytes(&raw).map_err(|why| {
            format!(
                "{} Selection V4 record at byte {location} is invalid: {why}",
                path.display()
            )
        })?;
        indexes.insert(
            receipt,
            &format!("{} Selection V4 record at byte {location}", path.display()),
        )?;
    }
    Ok(indexes)
}

fn validate_receipt_limit(max_receipts: usize) -> Result<(), SelectionV4Refusal> {
    if max_receipts == 0 {
        Err("Selection V4 receipt limit must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn receipt_capacity(len: u64, max_receipts: usize) -> Result<usize, SelectionV4Refusal> {
    validate_receipt_limit(max_receipts)?;
    let payload = len.checked_sub(HEADER_BYTES_U64).ok_or_else(|| {
        format!("Selection V4 ledger is shorter than its {HEADER_BYTES_U64}-byte header")
    })?;
    let count = payload / SELECTION_V4_STRIDE;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("Selection V4 receipt count {count} does not fit this machine"))?;
    if capacity > max_receipts {
        return Err(format!(
            "Selection V4 ledger contains {capacity} receipt(s), above caller maximum {max_receipts}"
        ));
    }
    Ok(capacity)
}

fn validate_file_length(len: u64) -> Result<(), SelectionV4Refusal> {
    if len < HEADER_BYTES_U64 {
        return Err(format!(
            "Selection V4 ledger is {len} bytes, shorter than its {HEADER_BYTES_U64}-byte header"
        ));
    }
    let payload = len.saturating_sub(HEADER_BYTES_U64);
    if !payload.is_multiple_of(SELECTION_V4_STRIDE) {
        return Err(format!(
            "Selection V4 ledger has {payload} bytes after its header, not a multiple of stride {SELECTION_V4_STRIDE}"
        ));
    }
    Ok(())
}

fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<(), SelectionV4Refusal> {
    let mut decoder = Decoder::new(header);
    if decoder.take(8)? != MAGIC_V4 {
        return Err(
            "Selection V4 ledger magic is unknown; no migration fallback exists".to_owned(),
        );
    }
    if decoder.u32()? != VERSION_V4 {
        return Err("Selection V4 ledger version is unknown".to_owned());
    }
    if decoder.u32()? != HEADER_BYTES_U32 {
        return Err("Selection V4 ledger header length is noncanonical".to_owned());
    }
    if decoder.u32()? != STRIDE_BYTES_U32_V4 {
        return Err("Selection V4 ledger stride is noncanonical".to_owned());
    }
    if decoder.u32()? != REQUESTED_TOP_V4 {
        return Err("Selection V4 ledger Top-N is not exactly 25".to_owned());
    }
    if decoder.u64()? != SCORE_SCALE {
        return Err("Selection V4 ledger score scale is noncanonical".to_owned());
    }
    decoder.zeros(8, "ledger header reserve")?;
    decoder.finish()
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

fn validated_generation(
    file: &File,
    path: &Path,
    expected_len: u64,
) -> Result<FileGeneration, SelectionV4Refusal> {
    let generation = file_generation(file, path)?;
    if generation.len != expected_len {
        return Err(format!(
            "{} changed from just-validated byte length {expected_len} to {}",
            path.display(),
            generation.len
        ));
    }
    Ok(generation)
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, SelectionV4Refusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionV4Refusal> {
    let held = FileGeneration {
        len: held.len(),
        platform: PlatformGeneration {
            device: held.dev(),
            inode: held.ino(),
            modified_seconds: held.mtime(),
            modified_nanoseconds: held.mtime_nsec(),
            changed_seconds: held.ctime(),
            changed_nanoseconds: held.ctime_nsec(),
        },
    };
    let named = FileGeneration {
        len: named.len(),
        platform: PlatformGeneration {
            device: named.dev(),
            inode: named.ino(),
            modified_seconds: named.mtime(),
            modified_nanoseconds: named.mtime_nsec(),
            changed_seconds: named.ctime(),
            changed_nanoseconds: named.ctime_nsec(),
        },
    };
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened Selection V4 ledger; replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its Selection V4 filesystem generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionV4Refusal> {
    fn of(metadata: &std::fs::Metadata, path: &Path) -> Result<FileGeneration, SelectionV4Refusal> {
        let volume_serial = metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "{} has no Windows volume serial; V4 stale detection fails closed",
                path.display()
            )
        })?;
        let file_index = metadata.file_index().ok_or_else(|| {
            format!(
                "{} has no Windows file index; V4 stale detection fails closed",
                path.display()
            )
        })?;
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial,
                file_index,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    }
    let held = of(held, path)?;
    let named = of(named, path)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened Selection V4 ledger; replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its Selection V4 filesystem generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, SelectionV4Refusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while its Selection V4 generation was measured",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

#[cfg(any(unix, windows))]
fn require_generation_unchanged(
    expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), SelectionV4Refusal> {
    if expected == observed {
        Ok(())
    } else {
        Err(format!(
            "{} generation changed after validation; reopen before V4 append",
            path.display()
        ))
    }
}

#[cfg(not(any(unix, windows)))]
fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), SelectionV4Refusal> {
    Err(format!(
        "{} is unchanged at {} bytes, but this target exposes no stable file identity; V4 append fails closed",
        path.display(),
        observed.len
    ))
}

const fn admission_index(status: AdmissionStatusV1) -> usize {
    match status {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    }
}

const fn execution_index(status: ExecutionSelectionStatusV2) -> usize {
    match status {
        ExecutionSelectionStatusV2::Authorized => 0,
        ExecutionSelectionStatusV2::PolicyRefused => 1,
    }
}

const fn admission_byte(status: AdmissionStatusV1) -> u8 {
    match status {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    }
}

const fn execution_byte(status: ExecutionSelectionStatusV2) -> u8 {
    match status {
        ExecutionSelectionStatusV2::Authorized => 0,
        ExecutionSelectionStatusV2::PolicyRefused => 1,
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(byte: u8) -> Result<InstrumentFamilyV1, SelectionV4Refusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "Selection V4 instrument-family byte {byte} is unknown"
        )),
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), SelectionV4Refusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!(
            "Selection V4 {name} is the reserved all-zero digest"
        ))
    } else {
        Ok(())
    }
}

fn checked_add(left: u64, right: u64, name: &str) -> Result<u64, SelectionV4Refusal> {
    left.checked_add(right)
        .ok_or_else(|| format!("Selection V4 {name} overflowed u64"))
}

fn checked_sum<const N: usize>(values: [u64; N], name: &str) -> Result<u64, SelectionV4Refusal> {
    values.into_iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| format!("Selection V4 {name} overflowed u64"))
    })
}

fn read_u32(bytes: &[u8]) -> Result<u32, SelectionV4Refusal> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "Selection V4 u32 field has the wrong width".to_owned())?;
    Ok(u32::from_le_bytes(array))
}

fn read_u64(bytes: &[u8]) -> Result<u64, SelectionV4Refusal> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| "Selection V4 u64 field has the wrong width".to_owned())?;
    Ok(u64::from_le_bytes(array))
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

struct Encoder<'a> {
    output: &'a mut [u8],
    cursor: usize,
}

impl<'a> Encoder<'a> {
    const fn new(output: &'a mut [u8]) -> Self {
        Self { output, cursor: 0 }
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), SelectionV4Refusal> {
        let end = self
            .cursor
            .checked_add(bytes.len())
            .ok_or_else(|| "Selection V4 encoder offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Selection V4 encoder exceeded its fixed record".to_owned())?;
        target.copy_from_slice(bytes);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), SelectionV4Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), SelectionV4Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), SelectionV4Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), SelectionV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Selection V4 reserve offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Selection V4 reserve exceeded its fixed record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionV4Refusal> {
        if self.cursor != self.output.len() {
            return Err(format!(
                "Selection V4 encoder wrote {} of {} fixed bytes",
                self.cursor,
                self.output.len()
            ));
        }
        Ok(())
    }
}

struct Decoder<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SelectionV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Selection V4 decoder offset overflowed usize".to_owned())?;
        let value = self
            .input
            .get(self.cursor..end)
            .ok_or_else(|| "Selection V4 decoder exceeded its fixed record".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SelectionV4Refusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "Selection V4 u8 field is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, SelectionV4Refusal> {
        read_u32(self.take(4)?)
    }

    fn u64(&mut self) -> Result<u64, SelectionV4Refusal> {
        read_u64(self.take(8)?)
    }

    fn array_32(&mut self) -> Result<[u8; 32], SelectionV4Refusal> {
        self.take(32)?
            .try_into()
            .map_err(|_| "Selection V4 digest field has the wrong width".to_owned())
    }

    fn zeros(&mut self, count: usize, name: &str) -> Result<(), SelectionV4Refusal> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            return Err(format!(
                "Selection V4 {name} contains nonzero reserved bytes"
            ));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionV4Refusal> {
        if self.cursor != self.input.len() {
            return Err(format!(
                "Selection V4 decoder consumed {} of {} fixed bytes",
                self.cursor,
                self.input.len()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "adversarial fixed-byte and authority fixtures must fail at their exact mutation"
)]
mod tests {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    use runner::topn::Weights;

    use super::*;
    use crate::population::{
        AdmissionStatusV1 as StoredAdmissionStatusV1, AdmissionV1, ClosureV1, ExitCoordinateV1,
    };

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone, Copy)]
    struct RowSpec {
        metric_seed: u64,
        admission: AdmissionStatusV1,
        execution: ExecutionSelectionStatusV2,
    }

    #[derive(Clone)]
    struct FakeAuthority {
        snapshot: SelectionAuthoritySnapshotV4,
        rows: Vec<SelectionJoinedRowV4>,
    }

    impl SelectionAuthorityViewV4 for FakeAuthority {
        fn snapshot(&self) -> &SelectionAuthoritySnapshotV4 {
            &self.snapshot
        }

        fn page(
            &mut self,
            offset: u64,
            limit: u64,
        ) -> Result<SelectionAuthorityPageV4, SelectionV4Refusal> {
            let start = usize::try_from(offset)
                .map_err(|_| "fake Selection V4 offset does not fit usize".to_owned())?;
            let requested = usize::try_from(limit)
                .map_err(|_| "fake Selection V4 limit does not fit usize".to_owned())?;
            let end = start
                .checked_add(requested)
                .ok_or_else(|| "fake Selection V4 page end overflowed".to_owned())?;
            let rows = self
                .rows
                .get(start..end.min(self.rows.len()))
                .ok_or_else(|| "fake Selection V4 page is outside its rows".to_owned())?
                .to_vec();
            Ok(SelectionAuthorityPageV4 {
                total: self.snapshot.population.row_count,
                offset,
                rows,
                population_v4_completion_digest: self.snapshot.population.completion_digest,
                admission_completion_digest: self.snapshot.admission.completion_digest,
                execution_completion_digest: self.snapshot.execution.completion_digest,
            })
        }
    }

    struct TempRoot {
        path: PathBuf,
    }

    fn temporary_path(label: &str) -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-selection-v4-{label}-{}-{sequence}",
            std::process::id()
        ))
    }

    impl TempRoot {
        fn new(label: &str) -> Self {
            let path = temporary_path(label);
            std::fs::create_dir_all(&path).expect("temporary Selection V4 root creates");
            Self { path }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn digest(label: &[u8], seed: u64) -> [u8; 32] {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex-selection-v4-test-digest\0");
        hasher.update(label);
        hasher.update(&seed.to_le_bytes());
        hasher.finalize()
    }

    fn policy() -> RankingPolicyV1 {
        RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy is valid")
    }

    fn cohort(ranking_policy_digest: [u8; 32]) -> SharedCohortIdentityV2 {
        let mut bytes = [0_u8; SHARED_COHORT_CANONICAL_LEN_V2];
        bytes[..16].copy_from_slice(b"brutex-cohort-v2");
        bytes[16..20].copy_from_slice(&2_u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&416_u32.to_le_bytes());
        for index in 0..13 {
            let start = 24 + index * 32;
            let value = if index == 6 {
                ranking_policy_digest
            } else {
                digest(b"cohort", u64::try_from(index).unwrap_or(u64::MAX) + 1)
            };
            bytes[start..start + 32].copy_from_slice(&value);
        }
        SharedCohortIdentityV2::from_canonical_bytes(&bytes)
            .expect("canonical V2 cohort fixture decodes")
    }

    fn metrics(seed: u64) -> TopMetricsV1 {
        TopMetricsV1 {
            drawdown: seed.saturating_add(10),
            worst_loss: seed.saturating_add(5),
            losing_rate_ppm: 166_666,
            losing_trades: 20,
            loss_ratio_ppm: Some(seed.saturating_add(100_000)),
            pessimistic_profit: i64::try_from(seed.saturating_mul(10).saturating_add(1_000))
                .unwrap_or(i64::MAX),
            winning_trades: 100,
            win_rate_ppm: 833_333,
            reward_to_risk_ppm: Some(seed.saturating_add(1_000_000)),
            average_win: seed.saturating_add(500),
            average_loss: seed.saturating_add(50),
            assurance_ppm: 900_000,
        }
    }

    fn canonical_live_mask(sequence: u64) -> [u64; 6] {
        let live_positions = runner::live_positions();
        let live_count =
            u64::try_from(live_positions.len()).expect("canonical live-position count fits u64");
        assert_ne!(live_count, 0, "canonical live-position authority is empty");
        let ordinal = usize::try_from(sequence % live_count).expect("live ordinal fits usize");
        let bit = live_positions
            .get(ordinal)
            .copied()
            .expect("canonical LIVE authority contains the requested ordinal");
        let word = usize::try_from(bit / 64).expect("canonical live word fits usize");
        let mut words = [0_u64; 6];
        *words
            .get_mut(word)
            .expect("canonical live bit belongs to the V1 durable mask") = 1_u64 << (bit % 64);
        let mask = runner::replay_mask::from_stored_words(words)
            .expect("generated Selection V4 fixture mask is canonical and live");
        assert_eq!(
            runner::replay_mask::stored_words(&mask),
            words,
            "every generated Selection V4 fixture mask must round-trip exactly"
        );
        words
    }

    fn population_row(
        family: InstrumentFamilyV1,
        population_id: [u8; 32],
        sequence: u64,
        metric_seed: u64,
    ) -> PopulationRowV1 {
        let mask_words = canonical_live_mask(sequence);
        PopulationRowV1 {
            population_id,
            sequence,
            strategy_digest: digest(
                match family {
                    InstrumentFamilyV1::Nifty => b"nifty-strategy",
                    InstrumentFamilyV1::BankNifty => b"bank-strategy",
                },
                sequence,
            ),
            mask_words,
            direction: if sequence.is_multiple_of(2) {
                TradeDirectionV1::Long
            } else {
                TradeDirectionV1::Short
            },
            instrument_family: family,
            closure: ClosureV1::Closed,
            rung_seconds: 60,
            support_hits: 100,
            exit: ExitCoordinateV1 {
                stop: None,
                target: None,
                tsl: None,
                ttp: None,
            },
            metrics: metrics(metric_seed),
            admission: AdmissionV1 {
                status: StoredAdmissionStatusV1::Admitted,
                reasons: 0,
                failed: 0,
                unmeasured: 0,
                refused: 0,
            },
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the fixture constructs one complete immutable three-authority snapshot and splitting it would hide binding terms"
    )]
    fn authority(
        family: InstrumentFamilyV1,
        specs: &[RowSpec],
        cohort: &SharedCohortIdentityV2,
        execution_salt: u64,
    ) -> FakeAuthority {
        let family_seed = match family {
            InstrumentFamilyV1::Nifty => 1_000,
            InstrumentFamilyV1::BankNifty => 2_000,
        };
        let population_id = digest(b"population", family_seed);
        let mut rows = Vec::new();
        let mut ordered = brutex_core::blake3::Hasher::new();
        ordered.update(b"brutex-selection-v4-test-population-order\0");
        for (index, spec) in specs.iter().enumerate() {
            let sequence = u64::try_from(index).expect("fixture index fits u64");
            let population = population_row(family, population_id, sequence, spec.metric_seed);
            let payload_digest = population
                .payload_digest()
                .expect("fixture population payload is canonical");
            ordered.update(&payload_digest);
            rows.push(SelectionJoinedRowV4 {
                population,
                admission: AdmissionSelectionRowV4 {
                    population_id,
                    sequence,
                    strategy_digest: population.strategy_digest,
                    population_payload_digest: payload_digest,
                    decision_digest: digest(
                        b"admission-row",
                        family_seed + sequence + u64::from(admission_byte(spec.admission)),
                    ),
                    status: spec.admission,
                },
                execution: ExecutionSelectionRowV2 {
                    population_id,
                    sequence,
                    strategy_digest: population.strategy_digest,
                    population_payload_digest: payload_digest,
                    population_v4_completion_digest: digest(
                        b"population-v4-completion",
                        family_seed,
                    ),
                    admission_completion_digest: digest(b"admission-completion", family_seed),
                    admission_status: spec.admission,
                    disposition_digest: digest(
                        b"execution-row",
                        family_seed
                            + sequence
                            + execution_salt
                            + u64::from(execution_byte(spec.execution)),
                    ),
                    status: spec.execution,
                },
            });
        }
        let admission_counts =
            specs
                .iter()
                .fold(AdmissionAuthorityCountsV4::default(), |mut counts, spec| {
                    match spec.admission {
                        AdmissionStatusV1::Admitted => counts.admitted += 1,
                        AdmissionStatusV1::Rejected => counts.rejected += 1,
                        AdmissionStatusV1::Unmeasured => counts.unmeasured += 1,
                        AdmissionStatusV1::Refused => counts.refused += 1,
                    }
                    counts
                });
        let execution_counts =
            specs
                .iter()
                .fold(ExecutionAuthorityCountsV2::default(), |mut counts, spec| {
                    match spec.execution {
                        ExecutionSelectionStatusV2::Authorized => counts.authorized += 1,
                        ExecutionSelectionStatusV2::PolicyRefused => counts.policy_refused += 1,
                    }
                    counts
                });
        let mut admission_execution_counts = [[0_u64; 2]; 4];
        for spec in specs {
            admission_execution_counts[admission_index(spec.admission)]
                [execution_index(spec.execution)] += 1;
        }
        let row_count = u64::try_from(specs.len()).expect("fixture row count fits u64");
        let population_completion = digest(b"population-v4-completion", family_seed);
        let snapshot = SelectionAuthoritySnapshotV4 {
            population: PopulationAuthoritySnapshotV4 {
                family,
                population_id,
                rung_seconds: 60,
                row_count,
                ordered_row_digest: ordered.finalize(),
                completion_digest: population_completion,
                ranking_policy_digest: cohort.ranking_policy_digest(),
                cohort: *cohort,
            },
            admission: AdmissionAuthoritySnapshotV4 {
                population_id,
                population_v4_completion_digest: population_completion,
                completion_digest: digest(b"admission-completion", family_seed),
                decision_count: row_count,
                counts: admission_counts,
            },
            execution: ExecutionAuthoritySnapshotV2 {
                population_id,
                population_v4_completion_digest: population_completion,
                admission_completion_digest: digest(b"admission-completion", family_seed),
                completion_digest: digest(b"execution-v2-completion", family_seed + execution_salt),
                disposition_count: row_count,
                counts: execution_counts,
                admission_execution_counts,
            },
        };
        FakeAuthority { snapshot, rows }
    }

    fn receipt_with_salt(execution_salt: u64) -> SelectionReceiptV4 {
        let ranking = policy();
        let shared = cohort(ranking.digest());
        let specs = [
            RowSpec {
                metric_seed: 10,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
            RowSpec {
                metric_seed: 20,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::PolicyRefused,
            },
        ];
        let mut nifty = authority(InstrumentFamilyV1::Nifty, &specs, &shared, execution_salt);
        let mut bank = authority(InstrumentFamilyV1::BankNifty, &[], &shared, execution_salt);
        SelectionReceiptV4::from_authorities(&mut nifty, &mut bank, ranking)
            .expect("fixture Selection V4 derives")
    }

    #[test]
    fn final_eligibility_is_the_exact_authority_conjunction() {
        let ranking = policy();
        let shared = cohort(ranking.digest());
        let specs = [
            RowSpec {
                metric_seed: 1,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
            RowSpec {
                metric_seed: 2,
                admission: AdmissionStatusV1::Rejected,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
            RowSpec {
                metric_seed: 3,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::PolicyRefused,
            },
            RowSpec {
                metric_seed: 4,
                admission: AdmissionStatusV1::Refused,
                execution: ExecutionSelectionStatusV2::PolicyRefused,
            },
        ];
        let mut nifty = authority(InstrumentFamilyV1::Nifty, &specs, &shared, 0);
        let mut bank = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        let receipt = SelectionReceiptV4::from_authorities(&mut nifty, &mut bank, ranking)
            .expect("complete authority conjunction derives");
        assert_eq!(receipt.population_proof().considered, 4);
        assert_eq!(receipt.population_proof().admitted, 1);
        assert_eq!(receipt.population_proof().refused, 3);
        assert_eq!(receipt.top_twenty_five().len(), 1);
        let accounting = receipt.populations()[0].accounting();
        assert_eq!(
            accounting.count(
                AdmissionStatusV1::Admitted,
                ExecutionSelectionStatusV2::Authorized
            ),
            1
        );
        assert_eq!(
            accounting.count(
                AdmissionStatusV1::Admitted,
                ExecutionSelectionStatusV2::PolicyRefused
            ),
            1
        );
    }

    #[test]
    fn policy_refused_extremes_do_not_change_normalisation_or_winners() {
        let ranking = policy();
        let shared = cohort(ranking.digest());
        let prefix = [
            RowSpec {
                metric_seed: 10,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
            RowSpec {
                metric_seed: 20,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
        ];
        let mut ordinary = prefix.to_vec();
        ordinary.push(RowSpec {
            metric_seed: 30,
            admission: AdmissionStatusV1::Admitted,
            execution: ExecutionSelectionStatusV2::PolicyRefused,
        });
        let mut extreme = prefix.to_vec();
        extreme.push(RowSpec {
            metric_seed: u64::try_from(i64::MAX / 20).unwrap_or(u64::MAX / 20),
            admission: AdmissionStatusV1::Admitted,
            execution: ExecutionSelectionStatusV2::PolicyRefused,
        });
        let mut nifty_a = authority(InstrumentFamilyV1::Nifty, &ordinary, &shared, 0);
        let mut bank_a = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        let mut nifty_b = authority(InstrumentFamilyV1::Nifty, &extreme, &shared, 0);
        let mut bank_b = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        let first = SelectionReceiptV4::from_authorities(&mut nifty_a, &mut bank_a, ranking)
            .expect("ordinary refused candidate derives");
        let second = SelectionReceiptV4::from_authorities(&mut nifty_b, &mut bank_b, ranking)
            .expect("extreme refused candidate derives");
        assert_eq!(first.top_twenty_five(), second.top_twenty_five());
        assert_ne!(first.selection_id(), second.selection_id());
    }

    #[test]
    fn top_ten_is_the_exact_prefix_of_the_authoritative_top_twenty_five() {
        let ranking = policy();
        let shared = cohort(ranking.digest());
        let specs = (0_u64..30)
            .map(|seed| RowSpec {
                metric_seed: seed,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            })
            .collect::<Vec<_>>();
        let mut nifty = authority(InstrumentFamilyV1::Nifty, &specs, &shared, 0);
        let mut bank = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        let receipt = SelectionReceiptV4::from_authorities(&mut nifty, &mut bank, ranking)
            .expect("thirty eligible rows derive");
        assert_eq!(receipt.top_twenty_five().len(), 25);
        assert_eq!(receipt.top_ten(), &receipt.top_twenty_five()[..10]);
    }

    #[test]
    fn execution_authority_is_a_selection_identity_term() {
        let first = receipt_with_salt(0);
        let second = receipt_with_salt(10_000);
        assert_eq!(first.top_twenty_five(), second.top_twenty_five());
        assert_ne!(first.selection_id(), second.selection_id());
        assert_ne!(
            first.populations()[0].execution_v2_completion_digest(),
            second.populations()[0].execution_v2_completion_digest()
        );
    }

    #[test]
    fn missing_and_duplicate_population_rows_fail_closed() {
        let ranking = policy();
        let shared = cohort(ranking.digest());
        let specs = [
            RowSpec {
                metric_seed: 1,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
            RowSpec {
                metric_seed: 2,
                admission: AdmissionStatusV1::Admitted,
                execution: ExecutionSelectionStatusV2::Authorized,
            },
        ];
        let mut missing = authority(InstrumentFamilyV1::Nifty, &specs, &shared, 0);
        missing.rows.pop();
        let mut empty_bank = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        assert!(
            SelectionReceiptV4::from_authorities(&mut missing, &mut empty_bank, ranking).is_err()
        );

        let mut duplicate = authority(InstrumentFamilyV1::Nifty, &specs, &shared, 0);
        duplicate.rows[1].population.sequence = 0;
        let mut empty_bank = authority(InstrumentFamilyV1::BankNifty, &[], &shared, 0);
        assert!(
            SelectionReceiptV4::from_authorities(&mut duplicate, &mut empty_bank, ranking).is_err()
        );
    }

    #[test]
    fn fixed_codec_refuses_tamper_and_recomputed_seal_with_stale_identity() {
        let receipt = receipt_with_salt(0);
        let raw = receipt.to_bytes().expect("Selection V4 encodes");
        assert_eq!(
            SelectionReceiptV4::from_bytes(&raw).expect("Selection V4 decodes"),
            receipt
        );

        let mut torn = raw;
        torn[SELECTION_V4_STRIDE_BYTES - 1] ^= 1;
        assert!(SelectionReceiptV4::from_bytes(&torn).is_err());

        let mut changed_authority = raw;
        let execution_completion_offset = 40 + 8 + 32 + 8 + 32 + 32 + 32;
        changed_authority[execution_completion_offset] ^= 1;
        let seal = brutex_core::blake3::hash(&changed_authority[..PAYLOAD_BYTES_V4]);
        changed_authority[PAYLOAD_BYTES_V4..].copy_from_slice(&seal);
        assert!(SelectionReceiptV4::from_bytes(&changed_authority).is_err());
    }

    #[test]
    fn writable_ledger_refuses_missing_or_nondirectory_root_without_recreation() {
        let missing_root = temporary_path("missing-authority-root");
        assert!(!missing_root.exists());
        let missing_error = SelectionLedgerV4::open(&missing_root, 4)
            .expect_err("a missing removable authority root must refuse");
        assert!(missing_error.contains("must already exist as a directory"));
        assert!(!missing_root.exists());

        let file_root = temporary_path("file-authority-root");
        File::create(&file_root).expect("non-directory authority fixture creates");
        let file_error = SelectionLedgerV4::open(&file_root, 4)
            .expect_err("a non-directory authority root must refuse");
        assert!(file_error.contains("is not a directory"));
        assert!(file_root.is_file());
        std::fs::remove_file(file_root).expect("non-directory authority fixture removes");
    }

    #[test]
    fn ledger_reopen_is_exact_and_v3_magic_or_torn_tail_never_migrates() {
        let root = TempRoot::new("reopen");
        let receipt = receipt_with_salt(0);
        {
            let mut ledger = SelectionLedgerV4::open(&root.path, 4).expect("V4 ledger creates");
            ledger.append(&receipt).expect("V4 receipt appends");
            assert!(ledger.append(&receipt).is_err());
        }
        {
            let ledger = SelectionLedgerV4::open_read(&root.path, 4).expect("V4 ledger reopens");
            assert_eq!(ledger.selections(), 1);
            assert_eq!(ledger.receipt(&receipt.selection_id()), Some(&receipt));
            assert_eq!(
                ledger
                    .latest_selection(receipt.cohort_digest(), receipt.rung_seconds())
                    .map(SelectionReceiptV4::selection_id),
                Some(receipt.selection_id())
            );
        }
        let path = SelectionLedgerV4::path(&root.path);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("V4 fixture file opens")
            .write_all(&[0xaa])
            .expect("torn V4 tail writes");
        assert!(SelectionLedgerV4::open_read(&root.path, 4).is_err());

        let mut old_header = [0_u8; HEADER_BYTES];
        let mut encoder = Encoder::new(&mut old_header);
        encoder.bytes(b"BRUTXSL3").expect("old magic fits");
        encoder.u32(3).expect("old version fits");
        encoder
            .zeros(HEADER_BYTES - 12)
            .expect("old header reserve fits");
        encoder.finish().expect("old header is fixed width");
        assert!(validate_header(&old_header).is_err());
    }

    #[test]
    fn externally_extended_writer_must_reopen_before_append() {
        let root = TempRoot::new("stale");
        let first = receipt_with_salt(0);
        let second = receipt_with_salt(20_000);
        let mut stale = SelectionLedgerV4::open(&root.path, 4).expect("first writer opens");
        let mut current = SelectionLedgerV4::open(&root.path, 4).expect("second writer opens");
        current.append(&first).expect("current writer appends");
        assert!(stale.append(&second).is_err());
        drop(stale);
        drop(current);
        let reopened = SelectionLedgerV4::open_read(&root.path, 4).expect("ledger reopens");
        assert_eq!(reopened.selections(), 1);
        assert_eq!(reopened.receipt(&first.selection_id()), Some(&first));
    }
}
