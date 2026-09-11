//! Admission-authoritative global Top-25 receipts for one signal timeframe.
//!
//! Version three does not reinterpret either earlier selection format.  It has
//! its own magic, path, header, payload, stride and content-derived identity.
//! Its two population references bind both the complete V4 population receipt
//! and the receipt-last admission completion.  Construction therefore streams
//! only [`AdmissionAuthoritativeLedger`] pages and derives admission from each
//! recomputed sidecar verdict; the historical summary on `PopulationRowV1` is
//! never ranking authority.
//!
//! # Cost
//!
//! Receipt construction is three O(NIFTY rows + BANKNIFTY rows) bounded-page
//! passes: extrema/proof, verified Top-25, then selected-row resolution.  RAM is
//! bounded by one 256-row joined page and 25 retained candidates.  Ledger open
//! is O(receipts); exact and latest lookup after open are one average-O(1) hash
//! probe.  None of construction, hashing, open or persistence is claimed O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.
//!
//! # Replay boundary
//!
//! A V3 selection proves which population rows won.  Population V1 rows do not
//! carry a decodable runner `Horizon` or full parameter record, so this receipt
//! alone must not be used to reconstruct a replay universe.  A replay caller
//! must additionally require a separate append-only parameter/grid authority.

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

use crate::admission_join::{
    AdmissionAuthoritativeLedger, AdmissionAuthoritativeRowV1, AdmissionAuthorityIdentityV1,
    AdmissionAuthoritySnapshotV1,
};
use crate::admission_store::AdmissionCompletionReceiptV1;
use crate::population::{
    CompletionReceiptV2, InstrumentFamilyV1, MAX_PAGE_ROWS_V1, TopMetricsV1, TradeDirectionV1,
};
use crate::selection::{
    SELECTED_ENTRY_CANONICAL_LEN_V1, SHARED_COHORT_CANONICAL_LEN_V2, SelectedEntryV1,
    SharedCohortIdentityV2,
};

/// Operator-facing refusal from V3 selection derivation or persistence.
pub type SelectionV3Refusal = String;

const MAGIC_V3: [u8; 8] = *b"BRUTXSL3";
const VERSION_V3: u32 = 3;
const HEADER: u64 = 40;
const HEADER_BYTES: usize = 40;
const HEADER_BYTES_U32: u32 = 40;
const SEAL_BYTES: usize = 32;
const PAYLOAD_BYTES_V3: usize = 3_080;
const STRIDE_BYTES_U32_V3: u32 = 3_112;
const POPULATION_REFERENCE_BYTES_V3: usize = 144;
const REQUESTED_TOP_V3: u32 = 25;
const ID_DOMAIN_V3: &[u8] = b"brutex-global-selection-id-v3\0";
const CANONICAL_RUNGS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];

/// Bytes in one canonical V3 population reference.
pub const POPULATION_REFERENCE_CANONICAL_LEN_V3: usize = POPULATION_REFERENCE_BYTES_V3;
/// Bytes in the sealed V3 receipt payload, excluding its BLAKE3 seal.
pub const SELECTION_V3_PAYLOAD_BYTES: usize = PAYLOAD_BYTES_V3;
/// Bytes in one complete sealed V3 selection receipt.
pub const SELECTION_V3_STRIDE_BYTES: usize = PAYLOAD_BYTES_V3 + SEAL_BYTES;
/// Fixed on-disk stride of one V3 selection receipt.
pub const SELECTION_V3_STRIDE: u64 = 3_112;

const _: () = assert!(MAX_TOP == 25);
const _: () = assert!(HEADER == 40);
const _: () = assert!(HEADER_BYTES == 40);
const _: () = assert!(HEADER_BYTES_U32 == 40);
const _: () = assert!(POPULATION_REFERENCE_BYTES_V3 == 144);
const _: () = assert!(SHARED_COHORT_CANONICAL_LEN_V2 == 440);
const _: () = assert!(SELECTED_ENTRY_CANONICAL_LEN_V1 == 88);
const _: () = assert!(
    32 + 4
        + 4
        + (2 * POPULATION_REFERENCE_BYTES_V3)
        + (4 * 8)
        + 32
        + 32
        + SHARED_COHORT_CANONICAL_LEN_V2
        + 4
        + 4
        + (MAX_TOP * SELECTED_ENTRY_CANONICAL_LEN_V1)
        + 8
        == PAYLOAD_BYTES_V3
);
const _: () = assert!(PAYLOAD_BYTES_V3 + SEAL_BYTES == SELECTION_V3_STRIDE_BYTES);
const _: () = assert!(SELECTION_V3_STRIDE_BYTES == 3_112);
const _: () = assert!(SELECTION_V3_STRIDE == 3_112);
const _: () = assert!(STRIDE_BYTES_U32_V3 == 3_112);

/// One exact V4 population plus receipt-last admission authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_field_names,
    reason = "the three independently domain-separated digests are exact fixed-layout authority terms"
)]
pub struct PopulationReferenceV3 {
    family: InstrumentFamilyV1,
    population_id: [u8; 32],
    row_count: u64,
    ordered_row_digest: [u8; 32],
    population_v4_completion_digest: [u8; 32],
    admission_completion_digest: [u8; 32],
}

impl PopulationReferenceV3 {
    fn from_snapshot(
        snapshot: &AdmissionAuthoritySnapshotV1,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionV3Refusal> {
        let v4 = snapshot.population_v4();
        let v2 = v4.v3().v2();
        if v2.instrument_family != expected_family {
            return Err(format!(
                "selection V3 expected {expected_family:?}, but population {} is {:?}",
                hex(&v2.population_id),
                v2.instrument_family
            ));
        }
        let admission = snapshot.admission();
        let identity = snapshot.identity();
        let population_v4_completion_digest = v4.content_digest()?;
        let admission_completion_digest = admission.digest()?;
        if identity.population_v4_completion_digest != population_v4_completion_digest
            || identity.admission_completion_digest != admission_completion_digest
        {
            return Err(format!(
                "population {} admission join snapshot digests do not reproduce its exact receipts",
                hex(&v2.population_id)
            ));
        }
        if admission.population_id() != v2.population_id
            || admission.population_v4_completion_digest() != population_v4_completion_digest
            || admission.decision_count() != v2.row_count
        {
            return Err(format!(
                "population {} admission completion is not bound to the complete V4 row set",
                hex(&v2.population_id)
            ));
        }
        let reference = Self {
            family: v2.instrument_family,
            population_id: v2.population_id,
            row_count: v2.row_count,
            ordered_row_digest: v2.ordered_row_digest,
            population_v4_completion_digest,
            admission_completion_digest,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }

    fn validate(self, expected_family: InstrumentFamilyV1) -> Result<(), SelectionV3Refusal> {
        if self.family != expected_family {
            return Err(format!(
                "selection V3 population order is noncanonical: expected {expected_family:?}, found {:?}",
                self.family
            ));
        }
        require_digest("V3 population identity", &self.population_id)?;
        require_digest("V3 ordered-row digest", &self.ordered_row_digest)?;
        require_digest(
            "V3 population V4 completion digest",
            &self.population_v4_completion_digest,
        )?;
        require_digest(
            "V3 admission completion digest",
            &self.admission_completion_digest,
        )
    }

    /// Canonical instrument family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Complete V4 population identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Exact committed row/decision count.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Digest of all V4 row payloads in sequence order.
    #[must_use]
    pub const fn ordered_row_digest(self) -> [u8; 32] {
        self.ordered_row_digest
    }

    /// Domain-separated digest of the exact V4 completion receipt.
    #[must_use]
    pub const fn population_v4_completion_digest(self) -> [u8; 32] {
        self.population_v4_completion_digest
    }

    /// Domain-separated digest of the exact admission completion receipt.
    #[must_use]
    pub const fn admission_completion_digest(self) -> [u8; 32] {
        self.admission_completion_digest
    }

    fn authority(self) -> AdmissionAuthorityIdentityV1 {
        AdmissionAuthorityIdentityV1 {
            population_v4_completion_digest: self.population_v4_completion_digest,
            admission_completion_digest: self.admission_completion_digest,
        }
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), SelectionV3Refusal> {
        encoder.u8(family_byte(self.family))?;
        encoder.zeros(7)?;
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        encoder.bytes(&self.population_v4_completion_digest)?;
        encoder.bytes(&self.admission_completion_digest)
    }

    fn decode(
        decoder: &mut Decoder<'_>,
        expected_family: InstrumentFamilyV1,
    ) -> Result<Self, SelectionV3Refusal> {
        let family = decode_family(decoder.u8()?)?;
        decoder.zeros(7, "V3 population-reference reserve")?;
        let reference = Self {
            family,
            population_id: decoder.array_32()?,
            row_count: decoder.u64()?,
            ordered_row_digest: decoder.array_32()?,
            population_v4_completion_digest: decoder.array_32()?,
            admission_completion_digest: decoder.array_32()?,
        };
        reference.validate(expected_family)?;
        Ok(reference)
    }
}

/// Durable global Top-25 derived only from admission-authoritative joined rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionReceiptV3 {
    selection_id: [u8; 32],
    rung_seconds: u32,
    populations: [PopulationReferenceV3; 2],
    proof: PopulationProof,
    ranking_policy_digest: [u8; 32],
    cohort: SharedCohortIdentityV2,
    selected: Vec<SelectedEntryV1>,
}

/// Test-only canonical inputs for one population reference in a V3 receipt.
///
/// This keeps fixture construction behind the same semantic validator as the
/// durable decoder without exposing a production constructor for authority
/// identities.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct CanonicalSelectionPopulationFixtureV3 {
    pub family: InstrumentFamilyV1,
    pub population_id: [u8; 32],
    pub row_count: u64,
    pub ordered_row_digest: [u8; 32],
    pub population_v4_completion_digest: [u8; 32],
    pub admission_completion_digest: [u8; 32],
}

/// Builds one fully derived, semantically validated V3 receipt for tests.
#[cfg(test)]
pub(crate) fn canonical_selection_receipt_fixture_v3(
    rung_seconds: u32,
    cohort: &SharedCohortIdentityV2,
    populations: &[CanonicalSelectionPopulationFixtureV3; 2],
    selected: Vec<SelectedEntryV1>,
) -> Result<SelectionReceiptV3, SelectionV3Refusal> {
    let [nifty, bank_nifty] = *populations;
    let references = [
        PopulationReferenceV3 {
            family: nifty.family,
            population_id: nifty.population_id,
            row_count: nifty.row_count,
            ordered_row_digest: nifty.ordered_row_digest,
            population_v4_completion_digest: nifty.population_v4_completion_digest,
            admission_completion_digest: nifty.admission_completion_digest,
        },
        PopulationReferenceV3 {
            family: bank_nifty.family,
            population_id: bank_nifty.population_id,
            row_count: bank_nifty.row_count,
            ordered_row_digest: bank_nifty.ordered_row_digest,
            population_v4_completion_digest: bank_nifty.population_v4_completion_digest,
            admission_completion_digest: bank_nifty.admission_completion_digest,
        },
    ];
    let considered = nifty
        .row_count
        .checked_add(bank_nifty.row_count)
        .ok_or_else(|| "canonical V3 fixture population count overflow".to_owned())?;
    let admitted = u64::try_from(selected.len())
        .map_err(|_| "canonical V3 fixture selected count does not fit u64".to_owned())?;
    if admitted > considered {
        return Err("canonical V3 fixture selects more rows than it considers".to_owned());
    }
    let mut proof_hasher = brutex_core::blake3::Hasher::new();
    proof_hasher.update(b"brutex.cli.selection-v3.canonical-test-proof\0");
    for entry in &selected {
        proof_hasher.update(&entry.canonical_bytes()?);
    }
    let mut receipt = SelectionReceiptV3 {
        selection_id: [0; 32],
        rung_seconds,
        populations: references,
        proof: PopulationProof {
            considered,
            admitted,
            refused: considered.saturating_sub(admitted),
            unmeasured: 0,
            ordered_digest: proof_hasher.finalize(),
        },
        ranking_policy_digest: cohort.ranking_policy_digest(),
        cohort: *cohort,
        selected,
    };
    receipt.selection_id = receipt.derived_id()?;
    receipt.validate_semantics()?;
    Ok(receipt)
}

impl SelectionReceiptV3 {
    /// Recomputes the exact global Top-25 from two joined V4/admission ledgers.
    ///
    /// # Errors
    ///
    /// Refuses noncanonical family order, timeframe/cohort/policy mismatch,
    /// changed authority snapshots, malformed pages, receipt-count mismatch,
    /// two-pass disagreement, ambiguous resolution or invalid fixed fields.
    pub fn from_admission_authorities(
        nifty: &mut AdmissionAuthoritativeLedger,
        bank_nifty: &mut AdmissionAuthoritativeLedger,
        policy: RankingPolicyV1,
    ) -> Result<Self, SelectionV3Refusal> {
        if nifty.population_id() == bank_nifty.population_id() {
            return Err(
                "NIFTY and BANKNIFTY admission authorities cannot share one population identity"
                    .to_owned(),
            );
        }
        let nifty_snapshot = nifty.snapshot().clone();
        let bank_snapshot = bank_nifty.snapshot().clone();
        let nifty_v4 = nifty_snapshot.population_v4();
        let bank_v4 = bank_snapshot.population_v4();
        let nifty_v2 = nifty_v4.v3().v2();
        let bank_v2 = bank_v4.v3().v2();
        if nifty_v2.rung_seconds != bank_v2.rung_seconds {
            return Err(format!(
                "global V3 selection requires one timeframe, but NIFTY is {}s and BANKNIFTY is {}s",
                nifty_v2.rung_seconds, bank_v2.rung_seconds
            ));
        }
        let ranking_policy_digest = policy.digest();
        for (name, receipt, admission) in [
            ("NIFTY", nifty_v2, nifty_snapshot.admission()),
            ("BANKNIFTY", bank_v2, bank_snapshot.admission()),
        ] {
            if receipt.identities.ranking_policy_digest != ranking_policy_digest {
                return Err(format!(
                    "{name} V4 ranking-policy identity differs from the requested canonical policy"
                ));
            }
            if receipt.identities.admission_policy_digest != admission.policy().digest() {
                return Err(format!(
                    "{name} V4 admission-policy identity differs from its sidecar authority"
                ));
            }
        }
        let cohort = SharedCohortIdentityV2::from_v4_receipts(&nifty_v4, &bank_v4)?;
        let references = [
            PopulationReferenceV3::from_snapshot(&nifty_snapshot, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV3::from_snapshot(&bank_snapshot, InstrumentFamilyV1::BankNifty)?,
        ];

        let mut extrema = Extrema::default();
        let mut first_pass = PopulationPass::new();
        visit_both_authorities(nifty, bank_nifty, &references, |_, _joined, candidate| {
            extrema.observe(&candidate);
            first_pass.observe(&candidate);
            Ok(())
        })?;
        let proof = first_pass.finish();
        validate_proof_against_authorities(
            proof,
            &nifty_v2,
            &bank_v2,
            nifty_snapshot.admission(),
            bank_snapshot.admission(),
        )?;

        let mut keeper = VerifiedKeeper::new(MAX_TOP, policy, extrema, proof)
            .map_err(|why| format!("global V3 Top-25 could not start: {why:?}"))?;
        visit_both_authorities(nifty, bank_nifty, &references, |_, _joined, candidate| {
            keeper
                .offer(candidate)
                .map_err(|why| format!("global V3 Top-25 candidate was refused: {why:?}"))
        })?;
        let selection = keeper
            .finish()
            .map_err(|why| format!("global V3 Top-25 did not reproduce its first pass: {why:?}"))?;
        validate_selection_reconciliation(&selection, proof)?;
        let selected = resolve_selected(nifty, bank_nifty, &references, &selection)?;

        let mut receipt = Self {
            selection_id: [0; 32],
            rung_seconds: nifty_v2.rung_seconds,
            populations: references,
            proof,
            ranking_policy_digest,
            cohort,
            selected,
        };
        receipt.selection_id = receipt.derived_id()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }

    /// Fully rebuilds and byte-compares this receipt against current authorities.
    ///
    /// # Errors
    ///
    /// Every construction refusal, or any field differing from the rebuilt
    /// receipt, including either source completion digest.
    pub fn verify_against_authorities(
        &self,
        nifty: &mut AdmissionAuthoritativeLedger,
        bank_nifty: &mut AdmissionAuthoritativeLedger,
        policy: RankingPolicyV1,
    ) -> Result<(), SelectionV3Refusal> {
        let rebuilt = Self::from_admission_authorities(nifty, bank_nifty, policy)?;
        if rebuilt != *self {
            return Err(
                "selection V3 differs from a full replay of its admission-authoritative populations"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Content-derived V3 selection identity.
    #[must_use]
    pub const fn selection_id(&self) -> [u8; 32] {
        self.selection_id
    }

    /// Shared signal timeframe.
    #[must_use]
    pub const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    /// Canonical `[NIFTY, BANKNIFTY]` admission-authoritative references.
    #[must_use]
    pub const fn populations(&self) -> &[PopulationReferenceV3; 2] {
        &self.populations
    }

    /// Exact combined first-pass population proof.
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

    /// Strongest-first Top-25, or every admitted row when fewer exist.
    #[must_use]
    pub fn top_twenty_five(&self) -> &[SelectedEntryV1] {
        &self.selected
    }

    /// First ten rows of the exact persisted Top-25 ordering.
    #[must_use]
    pub fn top_ten(&self) -> &[SelectedEntryV1] {
        let end = self.selected.len().min(10);
        self.selected.get(..end).unwrap_or(&[])
    }

    fn discovery_key(&self) -> ([u8; 32], u32) {
        (self.cohort.digest(), self.rung_seconds)
    }

    fn validate_semantics(&self) -> Result<(), SelectionV3Refusal> {
        require_digest("V3 selection identity", &self.selection_id)?;
        require_digest("V3 ranking-policy digest", &self.ranking_policy_digest)?;
        if !CANONICAL_RUNGS.contains(&self.rung_seconds) {
            return Err(format!(
                "selection V3 timeframe {} seconds is outside the eight canonical intraday rungs",
                self.rung_seconds
            ));
        }
        require_digest(
            "V3 population-proof ordered digest",
            &self.proof.ordered_digest,
        )?;
        self.cohort.validate()?;
        if self.cohort.ranking_policy_digest() != self.ranking_policy_digest {
            return Err(
                "selection V3 ranking-policy digest differs from its cohort identity".to_owned(),
            );
        }
        let [nifty, bank_nifty] = self.populations;
        nifty.validate(InstrumentFamilyV1::Nifty)?;
        bank_nifty.validate(InstrumentFamilyV1::BankNifty)?;
        if nifty.population_id == bank_nifty.population_id {
            return Err("both V3 families reference the same population".to_owned());
        }
        let expected_considered = checked_add(
            nifty.row_count,
            bank_nifty.row_count,
            "combined V3 population row counts",
        )?;
        if self.proof.considered != expected_considered {
            return Err(format!(
                "V3 proof considered {}, but source row counts total {expected_considered}",
                self.proof.considered
            ));
        }
        if checked_add(
            self.proof.admitted,
            self.proof.refused,
            "V3 proof verdict counts",
        )? != self.proof.considered
        {
            return Err("V3 proof admitted + refused does not equal considered".to_owned());
        }
        if self.proof.unmeasured > self.proof.considered {
            return Err("V3 proof undefined-metric count exceeds considered".to_owned());
        }
        let expected_selected = usize::try_from(self.proof.admitted)
            .unwrap_or(usize::MAX)
            .min(MAX_TOP);
        if self.selected.len() != expected_selected {
            return Err(format!(
                "selection V3 has {} row(s), but admitted={} requires exactly {expected_selected}",
                self.selected.len(),
                self.proof.admitted
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
                    return Err("selection V3 repeats one population row".to_owned());
                }
                if left.strategy_digest() == right.strategy_digest() {
                    return Err("selection V3 repeats one semantic strategy digest".to_owned());
                }
            }
        }
        if self.selected.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.score() < b.score())
        }) {
            return Err("selection V3 scores are not strongest-first".to_owned());
        }
        if self.derived_id()? != self.selection_id {
            return Err("selection V3 identity does not match its canonical content".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], SelectionV3Refusal> {
        let payload = self.payload_with_id([0; 32])?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(ID_DOMAIN_V3);
        hasher.update(
            payload
                .get(32..)
                .ok_or_else(|| "selection V3 identity payload is absent".to_owned())?,
        );
        Ok(hasher.finalize())
    }

    fn payload_with_id(
        &self,
        selection_id: [u8; 32],
    ) -> Result<[u8; PAYLOAD_BYTES_V3], SelectionV3Refusal> {
        let mut payload = [0_u8; PAYLOAD_BYTES_V3];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&selection_id)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u32(REQUESTED_TOP_V3)?;
        let [nifty, bank_nifty] = self.populations;
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
                .map_err(|_| "selected V3 row count does not fit u32".to_owned())?,
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

    fn to_bytes(&self) -> Result<[u8; SELECTION_V3_STRIDE_BYTES], SelectionV3Refusal> {
        self.validate_semantics()?;
        let payload = self.payload_with_id(self.selection_id)?;
        let mut raw = [0_u8; SELECTION_V3_STRIDE_BYTES];
        raw.get_mut(..PAYLOAD_BYTES_V3)
            .ok_or_else(|| "selection V3 payload slot is absent".to_owned())?
            .copy_from_slice(&payload);
        raw.get_mut(PAYLOAD_BYTES_V3..)
            .ok_or_else(|| "selection V3 seal slot is absent".to_owned())?
            .copy_from_slice(&brutex_core::blake3::hash(&payload));
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; SELECTION_V3_STRIDE_BYTES]) -> Result<Self, SelectionV3Refusal> {
        let payload = raw
            .get(..PAYLOAD_BYTES_V3)
            .ok_or_else(|| "selection V3 payload is absent".to_owned())?;
        let stored_seal = raw
            .get(PAYLOAD_BYTES_V3..)
            .ok_or_else(|| "selection V3 seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("selection V3 receipt failed its complete BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let selection_id = decoder.array_32()?;
        let rung_seconds = decoder.u32()?;
        let requested = decoder.u32()?;
        if requested != REQUESTED_TOP_V3 {
            return Err(format!(
                "selection V3 requests {requested}; version three requires exactly {REQUESTED_TOP_V3}"
            ));
        }
        let populations = [
            PopulationReferenceV3::decode(&mut decoder, InstrumentFamilyV1::Nifty)?,
            PopulationReferenceV3::decode(&mut decoder, InstrumentFamilyV1::BankNifty)?,
        ];
        let proof = PopulationProof {
            considered: decoder.u64()?,
            admitted: decoder.u64()?,
            refused: decoder.u64()?,
            unmeasured: decoder.u64()?,
            ordered_digest: decoder.array_32()?,
        };
        let ranking_policy_digest = decoder.array_32()?;
        let cohort_bytes = decoder.take(SHARED_COHORT_CANONICAL_LEN_V2)?;
        let cohort = SharedCohortIdentityV2::from_canonical_bytes(cohort_bytes)?;
        let selected_count = usize::try_from(decoder.u32()?)
            .map_err(|_| "selected V3 count does not fit this machine".to_owned())?;
        if selected_count > MAX_TOP {
            return Err(format!(
                "selection V3 stores {selected_count} rows above fixed maximum {MAX_TOP}"
            ));
        }
        decoder.zeros(4, "selection V3 selected-count reserve")?;
        let mut selected = Vec::new();
        selected.try_reserve_exact(selected_count).map_err(|why| {
            format!("selection V3 could not reserve {selected_count} bounded rows: {why}")
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
            "unused selection V3 entry slots",
        )?;
        decoder.zeros(8, "selection V3 trailing reserve")?;
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

fn visit_both_authorities<F>(
    nifty: &mut AdmissionAuthoritativeLedger,
    bank_nifty: &mut AdmissionAuthoritativeLedger,
    references: &[PopulationReferenceV3; 2],
    mut visit: F,
) -> Result<(), SelectionV3Refusal>
where
    F: FnMut(
        PopulationReferenceV3,
        AdmissionAuthoritativeRowV1,
        Candidate,
    ) -> Result<(), SelectionV3Refusal>,
{
    let [nifty_reference, bank_reference] = *references;
    visit_one_authority(nifty, nifty_reference, &mut visit)?;
    visit_one_authority(bank_nifty, bank_reference, &mut visit)
}

fn visit_one_authority<F>(
    authority: &mut AdmissionAuthoritativeLedger,
    reference: PopulationReferenceV3,
    visit: &mut F,
) -> Result<(), SelectionV3Refusal>
where
    F: FnMut(
        PopulationReferenceV3,
        AdmissionAuthoritativeRowV1,
        Candidate,
    ) -> Result<(), SelectionV3Refusal>,
{
    validate_authority_reference(authority, reference)?;
    let mut offset = 0_u64;
    while offset < reference.row_count {
        let page = authority.page(offset, MAX_PAGE_ROWS_V1)?;
        if page.total != reference.row_count
            || page.offset != offset
            || page.authority != reference.authority()
            || page.rows.is_empty()
        {
            return Err(format!(
                "admission-authoritative population {} returned a noncanonical page at offset {offset}",
                hex(&reference.population_id)
            ));
        }
        for joined in page.rows {
            let row = joined.population;
            if row.population_id != reference.population_id
                || row.instrument_family != reference.family
                || row.sequence != offset
            {
                return Err(format!(
                    "joined population {} row at offset {offset} disagrees with its V3 reference",
                    hex(&reference.population_id)
                ));
            }
            let candidate = candidate_from_authoritative(&joined);
            visit(reference, joined, candidate)?;
            offset = offset
                .checked_add(1)
                .ok_or_else(|| "selection V3 joined-row offset overflowed u64".to_owned())?;
        }
    }
    validate_authority_reference(authority, reference)
}

fn validate_authority_reference(
    authority: &AdmissionAuthoritativeLedger,
    reference: PopulationReferenceV3,
) -> Result<(), SelectionV3Refusal> {
    if authority.population_id() != reference.population_id {
        return Err(format!(
            "admission authority for population {} was paired with reference {}",
            hex(&authority.population_id()),
            hex(&reference.population_id)
        ));
    }
    let snapshot_reference =
        PopulationReferenceV3::from_snapshot(authority.snapshot(), reference.family)?;
    if snapshot_reference != reference || authority.authority() != reference.authority() {
        return Err(format!(
            "population {} admission authority differs from the selection V3 snapshot",
            hex(&reference.population_id)
        ));
    }
    Ok(())
}

fn candidate_from_authoritative(joined: &AdmissionAuthoritativeRowV1) -> Candidate {
    let row = joined.population;
    Candidate {
        strategy_digest: StrategyDigest::new(row.strategy_digest),
        mask_words: row.mask_words,
        direction: match row.direction {
            TradeDirectionV1::Long => Direction::Long,
            TradeDirectionV1::Short => Direction::Short,
        },
        admitted: sidecar_status_is_admitted(joined.admission.verdict().status()),
        metrics: metrics_from_row(row.metrics),
    }
}

const fn sidecar_status_is_admitted(status: AdmissionStatusV1) -> bool {
    matches!(status, AdmissionStatusV1::Admitted)
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

fn validate_proof_against_authorities(
    proof: PopulationProof,
    nifty_v2: &CompletionReceiptV2,
    bank_v2: &CompletionReceiptV2,
    nifty_admission: &AdmissionCompletionReceiptV1,
    bank_admission: &AdmissionCompletionReceiptV1,
) -> Result<(), SelectionV3Refusal> {
    let considered = checked_add(
        nifty_admission.decision_count(),
        bank_admission.decision_count(),
        "admission decision counts",
    )?;
    let admitted = checked_add(
        nifty_admission.admitted_count(),
        bank_admission.admitted_count(),
        "admitted sidecar decisions",
    )?;
    let nifty_refused = checked_sum(
        [
            nifty_admission.rejected_count(),
            nifty_admission.unmeasured_count(),
            nifty_admission.refused_count(),
        ],
        "NIFTY non-admitted sidecar decisions",
    )?;
    let bank_refused = checked_sum(
        [
            bank_admission.rejected_count(),
            bank_admission.unmeasured_count(),
            bank_admission.refused_count(),
        ],
        "BANKNIFTY non-admitted sidecar decisions",
    )?;
    let refused = checked_add(
        nifty_refused,
        bank_refused,
        "non-admitted sidecar decisions",
    )?;
    let unmeasured = checked_add(
        nifty_v2.topn_undefined_rows,
        bank_v2.topn_undefined_rows,
        "undefined Top-N metric rows",
    )?;
    if nifty_admission.decision_count() != nifty_v2.row_count
        || bank_admission.decision_count() != bank_v2.row_count
    {
        return Err(
            "selection V3 admission decisions do not cover both exact V4 populations".to_owned(),
        );
    }
    if proof.considered != considered
        || proof.admitted != admitted
        || proof.refused != refused
        || proof.unmeasured != unmeasured
    {
        return Err(format!(
            "combined V3 population proof counts {}/{}/{}/{} do not reconcile to authoritative sidecar/V4 counts {considered}/{admitted}/{refused}/{unmeasured}",
            proof.considered, proof.admitted, proof.refused, proof.unmeasured
        ));
    }
    require_digest(
        "combined V3 ordered-population digest",
        &proof.ordered_digest,
    )
}

fn validate_selection_reconciliation(
    selection: &Selection,
    proof: PopulationProof,
) -> Result<(), SelectionV3Refusal> {
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
        return Err("ranking selection counts do not reproduce the combined V3 proof".to_owned());
    }
    let expected = usize::try_from(proof.admitted)
        .unwrap_or(usize::MAX)
        .min(MAX_TOP);
    if selection.rows.len() != expected {
        return Err(format!(
            "ranking kernel returned {} rows, but admitted={} requires {expected}",
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
    nifty: &mut AdmissionAuthoritativeLedger,
    bank_nifty: &mut AdmissionAuthoritativeLedger,
    references: &[PopulationReferenceV3; 2],
    selection: &Selection,
) -> Result<Vec<SelectedEntryV1>, SelectionV3Refusal> {
    let mut resolved = vec![None; selection.rows.len()];
    visit_both_authorities(
        nifty,
        bank_nifty,
        references,
        |reference, joined, candidate| {
            let row = joined.population;
            resolve_candidate(
                &mut resolved,
                selection,
                reference,
                row.sequence,
                row.strategy_digest,
                candidate,
            )
        },
    )?;
    resolved
        .into_iter()
        .enumerate()
        .map(|(rank, entry)| {
            entry.ok_or_else(|| {
                format!(
                    "global V3 selection rank {} has no admission-authoritative source row",
                    rank + 1
                )
            })
        })
        .collect()
}

fn resolve_candidate(
    resolved: &mut [Option<SelectedEntryV1>],
    selection: &Selection,
    reference: PopulationReferenceV3,
    row_sequence: u64,
    row_strategy_digest: [u8; 32],
    candidate: Candidate,
) -> Result<(), SelectionV3Refusal> {
    for (index, ranked) in selection.rows.iter().enumerate() {
        if candidate.strategy_digest != ranked.candidate.strategy_digest {
            continue;
        }
        if candidate != ranked.candidate || candidate.strategy_digest.bytes() != row_strategy_digest
        {
            return Err(format!(
                "strategy digest {} aliases different admission-authoritative candidate bytes",
                hex(&candidate.strategy_digest.bytes())
            ));
        }
        let slot = resolved
            .get_mut(index)
            .ok_or_else(|| "bounded V3 selected-entry slot is absent".to_owned())?;
        if slot.is_some() {
            return Err(format!(
                "strategy digest {} occurs more than once in the joined populations",
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
    nifty: PopulationReferenceV3,
    bank_nifty: PopulationReferenceV3,
) -> Result<(), SelectionV3Refusal> {
    require_digest("selected V3 population identity", &entry.population_id())?;
    require_digest("selected V3 strategy digest", &entry.strategy_digest())?;
    if entry.score() > SCORE_SCALE {
        return Err(format!(
            "selected V3 score {} exceeds fixed-point domain 0..={SCORE_SCALE}",
            entry.score()
        ));
    }
    let source = match entry.family() {
        InstrumentFamilyV1::Nifty => nifty,
        InstrumentFamilyV1::BankNifty => bank_nifty,
    };
    if entry.population_id() != source.population_id {
        return Err(format!(
            "selected V3 {:?} row names a population other than its canonical source",
            entry.family()
        ));
    }
    if entry.row_sequence() >= source.row_count {
        return Err(format!(
            "selected V3 {:?} row sequence {} is outside its {}-row population",
            entry.family(),
            entry.row_sequence(),
            source.row_count
        ));
    }
    Ok(())
}

/// Append-only fixed-stride V3 selection ledger.
#[derive(Debug)]
pub struct SelectionLedgerV3 {
    file: File,
    path: PathBuf,
    receipts: HashMap<[u8; 32], SelectionReceiptV3>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
    scanned: u64,
    generation: FileGeneration,
    writable: bool,
    max_receipts: usize,
}

impl SelectionLedgerV3 {
    /// Version-three ledger path. Earlier selection files are never consulted.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("global-selections-v3.bin")
    }

    /// Opens or creates the append-only V3 ledger under an explicit row bound.
    ///
    /// # Errors
    ///
    /// Refuses a zero bound, I/O/locking failure, wrong V3 header, ragged or
    /// torn record, malformed receipt, duplicate identity, or file above the
    /// caller-supplied receipt ceiling.
    pub fn open(root: &Path, max_receipts: usize) -> Result<Self, SelectionV3Refusal> {
        validate_receipt_limit(max_receipts)?;
        let path = Self::path(root);
        let parent = path
            .parent()
            .ok_or_else(|| "selection V3 ledger has no parent directory".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|why| {
            format!(
                "selection V3 ledger directory {} could not be created: {why}",
                parent.display()
            )
        })?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        Self::open_file(file, path, true, max_receipts)
    }

    /// Opens an existing V3 ledger read-only under an explicit receipt bound.
    ///
    /// # Errors
    ///
    /// Every structural refusal from [`Self::open`], plus absence or an empty
    /// read-only file. No earlier-format fallback is attempted.
    pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionV3Refusal> {
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
    ) -> Result<Self, SelectionV3Refusal> {
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
                    format!("{} V3 header could not be synced: {why}", path.display())
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
                "{} could not be unlocked after V3 open: {why}",
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

    /// Number of indexed V3 receipts.
    #[must_use]
    pub fn selections(&self) -> usize {
        self.order.len()
    }

    /// Selection identities in exact append/file order.
    #[must_use]
    pub fn selection_ids(&self) -> &[[u8; 32]] {
        &self.order
    }

    /// Average-O(1) exact V3 receipt lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn receipt(&self, selection_id: &[u8; 32]) -> Option<&SelectionReceiptV3> {
        self.receipts.get(selection_id)
    }

    /// Latest append for one exact shared cohort digest and signal rung.
    ///
    /// This never falls back to V1/V2, another cohort or another timeframe.
    #[must_use]
    pub fn latest_selection(
        &self,
        cohort_digest: [u8; 32],
        rung_seconds: u32,
    ) -> Option<&SelectionReceiptV3> {
        self.latest
            .get(&(cohort_digest, rung_seconds))
            .and_then(|selection_id| self.receipts.get(selection_id))
    }

    /// Appends and syncs one complete fixed-stride V3 receipt.
    ///
    /// # Errors
    ///
    /// Refuses read-only use, invalid or duplicate bytes, stale/replaced file
    /// generation, a caller bound, arithmetic failure, or any I/O/lock error.
    pub fn append(&mut self, receipt: &SelectionReceiptV3) -> Result<(), SelectionV3Refusal> {
        if !self.writable {
            return Err("a read-only selection V3 ledger cannot append".to_owned());
        }
        receipt.validate_semantics()?;
        self.file.lock().map_err(|why| {
            format!(
                "{} could not be locked for V3 append: {why}",
                self.path.display()
            )
        })?;
        let attempted = (|| {
            self.absorb_new()?;
            if let Some(existing) = self.receipts.get(&receipt.selection_id) {
                if existing == receipt {
                    return Err(format!(
                        "selection V3 {} is already present; duplicate identities are refused",
                        hex(&receipt.selection_id)
                    ));
                }
                return Err(format!(
                    "selection V3 {} already names different canonical bytes",
                    hex(&receipt.selection_id)
                ));
            }
            if self.order.len() >= self.max_receipts {
                return Err(format!(
                    "selection V3 ledger already contains its caller-supplied maximum of {} receipt(s)",
                    self.max_receipts
                ));
            }
            self.receipts
                .try_reserve(1)
                .map_err(|why| format!("selection V3 index could not reserve one slot: {why}"))?;
            self.order
                .try_reserve(1)
                .map_err(|why| format!("selection V3 order could not reserve one slot: {why}"))?;
            self.latest.try_reserve(1).map_err(|why| {
                format!("selection V3 latest index could not reserve one slot: {why}")
            })?;
            let raw = receipt.to_bytes()?;
            self.file
                .seek(SeekFrom::End(0))
                .map_err(|why| format!("selection V3 ledger could not seek to append: {why}"))?;
            self.file
                .write_all(&raw)
                .map_err(|why| format!("selection V3 receipt could not be appended: {why}"))?;
            self.file
                .sync_all()
                .map_err(|why| format!("selection V3 receipt could not be synced: {why}"))?;
            self.scanned = self
                .scanned
                .checked_add(SELECTION_V3_STRIDE)
                .ok_or_else(|| "selection V3 scanned length overflowed u64".to_owned())?;
            self.generation = validated_generation(&self.file, &self.path, self.scanned)?;
            let selection_id = receipt.selection_id;
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
            self.latest.insert(receipt.discovery_key(), selection_id);
            Ok(())
        })();
        let released = self.file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after V3 append: {why}",
                self.path.display()
            )
        });
        match (attempted, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
    }

    fn absorb_new(&mut self) -> Result<(), SelectionV3Refusal> {
        let observed = file_generation(&self.file, &self.path)?;
        let len = observed.len;
        validate_file_length(len)?;
        let total = receipt_capacity(len, self.max_receipts)?;
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|why| format!("selection V3 could not recheck its header: {why}"))?;
        let mut header = [0_u8; HEADER_BYTES];
        self.file
            .read_exact(&mut header)
            .map_err(|why| format!("selection V3 header could not be rechecked: {why}"))?;
        validate_header(&header)?;
        if len < self.scanned {
            return Err(format!(
                "selection V3 ledger shrank from {} to {len}; append-only history was violated",
                self.scanned
            ));
        }
        if len == self.scanned {
            require_generation_unchanged(self.generation, observed, &self.path)?;
            self.generation = observed;
            return Ok(());
        }
        let held = self.order.len();
        let new_count = total
            .checked_sub(held)
            .ok_or_else(|| "selection V3 count moved behind its in-memory index".to_owned())?;
        self.file
            .seek(SeekFrom::Start(self.scanned))
            .map_err(|why| format!("selection V3 could not seek to new receipts: {why}"))?;
        let added = scan_receipts(
            &mut self.file,
            &self.path,
            self.scanned,
            new_count,
            Some(&self.receipts),
        )?;
        self.receipts.try_reserve(new_count).map_err(|why| {
            format!("selection V3 index could not reserve {new_count} new slot(s): {why}")
        })?;
        self.order.try_reserve(new_count).map_err(|why| {
            format!("selection V3 order could not reserve {new_count} new slot(s): {why}")
        })?;
        self.latest.try_reserve(new_count).map_err(|why| {
            format!("selection V3 latest index could not reserve {new_count} new slot(s): {why}")
        })?;
        let SelectionIndexesV3 {
            receipts: added_receipts,
            order: added_order,
            latest: _,
        } = added;
        for selection_id in added_order {
            let receipt = added_receipts.get(&selection_id).ok_or_else(|| {
                "selection V3 scan produced an ordered ID without its receipt".to_owned()
            })?;
            self.latest.insert(receipt.discovery_key(), selection_id);
            self.receipts.insert(selection_id, receipt.clone());
            self.order.push(selection_id);
        }
        self.scanned = len;
        self.generation = validated_generation(&self.file, &self.path, len)?;
        Ok(())
    }
}

#[derive(Debug)]
struct SelectionIndexesV3 {
    receipts: HashMap<[u8; 32], SelectionReceiptV3>,
    order: Vec<[u8; 32]>,
    latest: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl SelectionIndexesV3 {
    fn with_capacity(capacity: usize) -> Result<Self, SelectionV3Refusal> {
        let mut receipts = HashMap::new();
        receipts.try_reserve(capacity).map_err(|why| {
            format!("selection V3 index could not reserve {capacity} slots: {why}")
        })?;
        let mut order = Vec::new();
        order.try_reserve_exact(capacity).map_err(|why| {
            format!("selection V3 order could not reserve {capacity} slots: {why}")
        })?;
        let mut latest = HashMap::new();
        latest.try_reserve(capacity).map_err(|why| {
            format!("selection V3 latest index could not reserve {capacity} slots: {why}")
        })?;
        Ok(Self {
            receipts,
            order,
            latest,
        })
    }

    fn insert(
        &mut self,
        receipt: SelectionReceiptV3,
        location: &str,
    ) -> Result<(), SelectionV3Refusal> {
        let selection_id = receipt.selection_id;
        if let Some(existing) = self.receipts.get(&selection_id) {
            if existing == &receipt {
                return Err(format!(
                    "{location} repeats selection V3 identity {}",
                    hex(&selection_id)
                ));
            }
            return Err(format!(
                "{location} reuses selection V3 identity {} for different bytes",
                hex(&selection_id)
            ));
        }
        self.latest.insert(receipt.discovery_key(), selection_id);
        self.receipts.insert(selection_id, receipt);
        self.order.push(selection_id);
        Ok(())
    }
}

fn write_header(file: &mut File) -> Result<(), SelectionV3Refusal> {
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&MAGIC_V3)?;
    encoder.u32(VERSION_V3)?;
    encoder.u32(HEADER_BYTES_U32)?;
    encoder.u32(STRIDE_BYTES_U32_V3)?;
    encoder.u32(REQUESTED_TOP_V3)?;
    encoder.u64(SCORE_SCALE)?;
    encoder.zeros(8)?;
    encoder.finish()?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("selection V3 header seek failed: {why}"))?;
    file.write_all(&header)
        .map_err(|why| format!("selection V3 header write failed: {why}"))
}

fn scan_file(
    file: &mut File,
    path: &Path,
    len: u64,
    max_receipts: usize,
) -> Result<SelectionIndexesV3, SelectionV3Refusal> {
    validate_file_length(len)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} V3 header could not be seeked: {why}", path.display()))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("{} V3 header could not be read: {why}", path.display()))?;
    validate_header(&header)?;
    let count = receipt_capacity(len, max_receipts)?;
    scan_receipts(file, path, HEADER, count, None)
}

fn scan_receipts(
    file: &mut File,
    path: &Path,
    start: u64,
    count: usize,
    existing: Option<&HashMap<[u8; 32], SelectionReceiptV3>>,
) -> Result<SelectionIndexesV3, SelectionV3Refusal> {
    let mut indexes = SelectionIndexesV3::with_capacity(count)?;
    for index in 0..count {
        let index_u64 = u64::try_from(index)
            .map_err(|_| "selection V3 scan index does not fit u64".to_owned())?;
        let location = index_u64
            .checked_mul(SELECTION_V3_STRIDE)
            .and_then(|offset| start.checked_add(offset))
            .ok_or_else(|| "selection V3 scan byte offset overflowed u64".to_owned())?;
        let mut raw = [0_u8; SELECTION_V3_STRIDE_BYTES];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} selection V3 record at byte {location} could not be read: {why}",
                path.display()
            )
        })?;
        let receipt = SelectionReceiptV3::from_bytes(&raw).map_err(|why| {
            format!(
                "{} selection V3 record at byte {location} is invalid: {why}",
                path.display()
            )
        })?;
        if let Some(previous) = existing.and_then(|held| held.get(&receipt.selection_id)) {
            if previous == &receipt {
                return Err(format!(
                    "{} repeats selection V3 identity {} at byte {location}",
                    path.display(),
                    hex(&receipt.selection_id)
                ));
            }
            return Err(format!(
                "{} reuses selection V3 identity {} for different bytes at byte {location}",
                path.display(),
                hex(&receipt.selection_id)
            ));
        }
        indexes.insert(
            receipt,
            &format!("{} selection V3 record at byte {location}", path.display()),
        )?;
    }
    Ok(indexes)
}

fn validate_receipt_limit(max_receipts: usize) -> Result<(), SelectionV3Refusal> {
    if max_receipts == 0 {
        Err("selection V3 receipt limit must be nonzero".to_owned())
    } else {
        Ok(())
    }
}

fn receipt_capacity(len: u64, max_receipts: usize) -> Result<usize, SelectionV3Refusal> {
    validate_receipt_limit(max_receipts)?;
    let payload = len
        .checked_sub(HEADER)
        .ok_or_else(|| format!("selection V3 ledger is shorter than its {HEADER}-byte header"))?;
    let count = payload / SELECTION_V3_STRIDE;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("selection V3 receipt count {count} does not fit this machine"))?;
    if capacity > max_receipts {
        return Err(format!(
            "selection V3 ledger contains {capacity} receipt(s), above caller maximum {max_receipts}"
        ));
    }
    Ok(capacity)
}

fn validate_file_length(len: u64) -> Result<(), SelectionV3Refusal> {
    if len < HEADER {
        return Err(format!(
            "selection V3 ledger is {len} bytes, shorter than its {HEADER}-byte header"
        ));
    }
    let payload = len.saturating_sub(HEADER);
    if !payload.is_multiple_of(SELECTION_V3_STRIDE) {
        return Err(format!(
            "selection V3 ledger has {payload} bytes after its header, not a multiple of stride {SELECTION_V3_STRIDE}"
        ));
    }
    Ok(())
}

fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<(), SelectionV3Refusal> {
    let mut decoder = Decoder::new(header);
    if decoder.take(8)? != MAGIC_V3 {
        return Err("selection V3 ledger magic is unknown".to_owned());
    }
    if decoder.u32()? != VERSION_V3 {
        return Err("selection V3 ledger version is unknown".to_owned());
    }
    if decoder.u32()? != HEADER_BYTES_U32 {
        return Err("selection V3 ledger header length is noncanonical".to_owned());
    }
    if decoder.u32()? != STRIDE_BYTES_U32_V3 {
        return Err("selection V3 ledger stride is noncanonical".to_owned());
    }
    if decoder.u32()? != REQUESTED_TOP_V3 {
        return Err("selection V3 ledger requested Top-N is not exactly 25".to_owned());
    }
    if decoder.u64()? != SCORE_SCALE {
        return Err("selection V3 ledger score scale is noncanonical".to_owned());
    }
    decoder.zeros(8, "selection V3 ledger header reserve")?;
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
) -> Result<FileGeneration, SelectionV3Refusal> {
    let generation = file_generation(file, path)?;
    if generation.len != expected_len {
        return Err(format!(
            "{} changed length from just-validated byte {expected_len} to {} before its generation could be retained",
            path.display(),
            generation.len
        ));
    }
    Ok(generation)
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, SelectionV3Refusal> {
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
) -> Result<FileGeneration, SelectionV3Refusal> {
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
            "{} no longer names the opened selection V3 ledger; replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its V3 filesystem generation was measured",
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
) -> Result<FileGeneration, SelectionV3Refusal> {
    fn of(metadata: &std::fs::Metadata, path: &Path) -> Result<FileGeneration, SelectionV3Refusal> {
        let volume_serial = metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "{} has no Windows volume serial; V3 stale detection fails closed",
                path.display()
            )
        })?;
        let file_index = metadata.file_index().ok_or_else(|| {
            format!(
                "{} has no Windows file index; V3 stale detection fails closed",
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
            "{} no longer names the opened selection V3 ledger; replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its V3 filesystem generation was measured",
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
) -> Result<FileGeneration, SelectionV3Refusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while its V3 generation was measured",
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
) -> Result<(), SelectionV3Refusal> {
    if expected == observed {
        return Ok(());
    }
    Err(format!(
        "{} kept length {} but its validated V3 generation changed; reopen before append",
        path.display(),
        observed.len
    ))
}

#[cfg(not(any(unix, windows)))]
fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), SelectionV3Refusal> {
    Err(format!(
        "{} is unchanged at {} bytes, but this target exposes no stable file identity; V3 append fails closed",
        path.display(),
        observed.len
    ))
}

fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn decode_family(byte: u8) -> Result<InstrumentFamilyV1, SelectionV3Refusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "selection V3 instrument-family byte {byte} is unknown; only 1 and 2 are canonical"
        )),
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), SelectionV3Refusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("selection {name} is the reserved all-zero digest"))
    } else {
        Ok(())
    }
}

fn checked_add(left: u64, right: u64, name: &str) -> Result<u64, SelectionV3Refusal> {
    left.checked_add(right)
        .ok_or_else(|| format!("selection V3 {name} overflow u64"))
}

fn checked_sum<const N: usize>(values: [u64; N], name: &str) -> Result<u64, SelectionV3Refusal> {
    values.into_iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| format!("selection V3 {name} overflow u64"))
    })
}

fn read_u32(bytes: &[u8]) -> Result<u32, SelectionV3Refusal> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "selection V3 u32 field has the wrong width".to_owned())?;
    Ok(u32::from_le_bytes(array))
}

fn read_u64(bytes: &[u8]) -> Result<u64, SelectionV3Refusal> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| "selection V3 u64 field has the wrong width".to_owned())?;
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
    fn new(output: &'a mut [u8]) -> Self {
        Self { output, cursor: 0 }
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), SelectionV3Refusal> {
        let end = self
            .cursor
            .checked_add(bytes.len())
            .ok_or_else(|| "selection V3 encoder offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "selection V3 encoder exceeded its fixed record".to_owned())?;
        target.copy_from_slice(bytes);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), SelectionV3Refusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), SelectionV3Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), SelectionV3Refusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), SelectionV3Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "selection V3 reserve offset overflowed usize".to_owned())?;
        let target = self
            .output
            .get_mut(self.cursor..end)
            .ok_or_else(|| "selection V3 reserve exceeded its fixed record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionV3Refusal> {
        if self.cursor != self.output.len() {
            return Err(format!(
                "selection V3 encoder wrote {} of {} fixed bytes",
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
    fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SelectionV3Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "selection V3 decoder offset overflowed usize".to_owned())?;
        let value = self
            .input
            .get(self.cursor..end)
            .ok_or_else(|| "selection V3 decoder exceeded its fixed record".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SelectionV3Refusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "selection V3 u8 field is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, SelectionV3Refusal> {
        read_u32(self.take(4)?)
    }

    fn u64(&mut self) -> Result<u64, SelectionV3Refusal> {
        read_u64(self.take(8)?)
    }

    fn array_32(&mut self) -> Result<[u8; 32], SelectionV3Refusal> {
        self.take(32)?
            .try_into()
            .map_err(|_| "selection V3 digest field has the wrong width".to_owned())
    }

    fn zeros(&mut self, count: usize, name: &str) -> Result<(), SelectionV3Refusal> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            return Err(format!(
                "selection V3 {name} contains nonzero reserved bytes"
            ));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), SelectionV3Refusal> {
        if self.cursor != self.input.len() {
            return Err(format!(
                "selection V3 decoder consumed {} of {} fixed bytes",
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
    reason = "adversarial fixed-byte fixtures must fail loudly at their exact mutation"
)]
mod tests {
    use std::io::{Seek as _, SeekFrom, Write as _};
    use std::sync::atomic::{AtomicU64, Ordering};

    use costs::fill::Direction;
    use runner::admission::AdmissionStatusV1;
    use runner::portfolio::StrategyDigest;
    use runner::topn::{
        Candidate, Extrema, MAX_TOP, Metrics, PopulationPass, PopulationProof, RankingPolicyV1,
        SCORE_SCALE, VerifiedKeeper, Weights,
    };

    use crate::population::InstrumentFamilyV1;
    use crate::selection::{SelectedEntryV1, SharedCohortIdentityV2};

    use super::{
        HEADER, HEADER_BYTES, MAGIC_V3, PAYLOAD_BYTES_V3, POPULATION_REFERENCE_BYTES_V3,
        PopulationReferenceV3, REQUESTED_TOP_V3, SELECTION_V3_STRIDE, SELECTION_V3_STRIDE_BYTES,
        STRIDE_BYTES_U32_V3, SelectionLedgerV3, SelectionReceiptV3, VERSION_V3, resolve_candidate,
        sidecar_status_is_admitted,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn root(name: &str) -> std::path::PathBuf {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-selection-v3-{}-{name}-{sequence}",
            std::process::id()
        ))
    }

    const fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn cohort(seed: u8) -> SharedCohortIdentityV2 {
        let mut bytes = [0_u8; 440];
        bytes[..16].copy_from_slice(b"brutex-cohort-v2");
        bytes[16..20].copy_from_slice(&2_u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&416_u32.to_le_bytes());
        for index in 0_u8..13 {
            let start = 24 + usize::from(index) * 32;
            bytes[start..start + 32].copy_from_slice(&digest(seed.wrapping_add(index)));
        }
        SharedCohortIdentityV2::from_canonical_bytes(&bytes).expect("canonical V2 cohort")
    }

    fn reference(family: InstrumentFamilyV1, seed: u8, row_count: u64) -> PopulationReferenceV3 {
        PopulationReferenceV3 {
            family,
            population_id: digest(seed),
            row_count,
            ordered_row_digest: digest(seed.wrapping_add(1)),
            population_v4_completion_digest: digest(seed.wrapping_add(2)),
            admission_completion_digest: digest(seed.wrapping_add(3)),
        }
    }

    fn receipt(seed: u8, rung_seconds: u32, admitted: u64) -> SelectionReceiptV3 {
        let cohort = cohort(seed.wrapping_add(30));
        let populations = [
            reference(InstrumentFamilyV1::Nifty, seed.wrapping_add(1), 20),
            reference(InstrumentFamilyV1::BankNifty, seed.wrapping_add(10), 20),
        ];
        let selected_count = usize::try_from(admitted).unwrap_or(usize::MAX).min(MAX_TOP);
        let mut selected = Vec::new();
        for index in 0..selected_count {
            let family = if index % 2 == 0 {
                InstrumentFamilyV1::Nifty
            } else {
                InstrumentFamilyV1::BankNifty
            };
            let source = match family {
                InstrumentFamilyV1::Nifty => populations[0],
                InstrumentFamilyV1::BankNifty => populations[1],
            };
            selected.push(SelectedEntryV1::new(
                family,
                source.population_id,
                u64::try_from(index / 2).expect("small row sequence"),
                digest(
                    seed.wrapping_add(80)
                        .wrapping_add(u8::try_from(index).expect("Top-25 index")),
                ),
                SCORE_SCALE.saturating_sub(u64::try_from(index).expect("Top-25 index")),
            ));
        }
        let mut value = SelectionReceiptV3 {
            selection_id: [0; 32],
            rung_seconds,
            populations,
            proof: PopulationProof {
                considered: 40,
                admitted,
                refused: 40_u64.saturating_sub(admitted),
                unmeasured: 0,
                ordered_digest: digest(seed.wrapping_add(20)),
            },
            ranking_policy_digest: cohort.ranking_policy_digest(),
            cohort,
            selected,
        };
        value.selection_id = value.derived_id().expect("content-derived V3 identity");
        value
            .validate_semantics()
            .expect("valid V3 receipt fixture");
        value
    }

    fn reseal(raw: &mut [u8; SELECTION_V3_STRIDE_BYTES]) {
        let seal = brutex_core::blake3::hash(&raw[..PAYLOAD_BYTES_V3]);
        raw[PAYLOAD_BYTES_V3..].copy_from_slice(&seal);
    }

    fn cleanup(path: &std::path::Path) {
        std::fs::remove_dir_all(path).expect("unique fixture cleanup");
    }

    #[test]
    fn exact_layout_roundtrips_and_reserved_or_seal_mutations_refuse() {
        assert_eq!(HEADER_BYTES, 40);
        assert_eq!(POPULATION_REFERENCE_BYTES_V3, 144);
        assert_eq!(PAYLOAD_BYTES_V3, 3_080);
        assert_eq!(SELECTION_V3_STRIDE_BYTES, 3_112);
        assert_eq!(SELECTION_V3_STRIDE, 3_112);
        let value = receipt(1, 60, 30);
        let raw = value.to_bytes().expect("encode V3 receipt");
        assert_eq!(u32::from_le_bytes(raw[32..36].try_into().unwrap()), 60);
        assert_eq!(
            u32::from_le_bytes(raw[36..40].try_into().unwrap()),
            REQUESTED_TOP_V3
        );
        assert_eq!(u32::from_le_bytes(raw[864..868].try_into().unwrap()), 25);
        assert_eq!(SelectionReceiptV3::from_bytes(&raw).unwrap(), value);

        let mut torn = raw;
        torn[100] ^= 1;
        assert!(
            SelectionReceiptV3::from_bytes(&torn)
                .unwrap_err()
                .contains("BLAKE3 seal")
        );

        let mut reserved = raw;
        reserved[41] = 1;
        reseal(&mut reserved);
        assert!(
            SelectionReceiptV3::from_bytes(&reserved)
                .unwrap_err()
                .contains("population-reference reserve")
        );

        let mut unused = receipt(2, 60, 3).to_bytes().unwrap();
        let first_unused_entry = 872 + 3 * 88;
        unused[first_unused_entry] = 1;
        reseal(&mut unused);
        assert!(
            SelectionReceiptV3::from_bytes(&unused)
                .unwrap_err()
                .contains("unused selection V3 entry slots")
        );
    }

    #[test]
    fn authority_digest_changes_identity_and_top_ten_is_exact_prefix() {
        let original = receipt(3, 300, 30);
        assert_eq!(original.top_twenty_five().len(), 25);
        assert_eq!(original.top_ten(), &original.top_twenty_five()[..10]);
        let mut changed = original.clone();
        changed.populations[0].admission_completion_digest = digest(254);
        changed.selection_id = changed.derived_id().unwrap();
        changed.validate_semantics().unwrap();
        assert_ne!(changed.selection_id(), original.selection_id());
        assert_ne!(
            changed.populations()[0].admission_completion_digest(),
            original.populations()[0].admission_completion_digest()
        );
    }

    #[test]
    fn empty_and_sub_twenty_five_populations_have_exact_cardinality() {
        let empty = receipt(4, 600, 0);
        assert!(empty.top_twenty_five().is_empty());
        assert!(empty.top_ten().is_empty());

        let seven = receipt(5, 600, 7);
        assert_eq!(seven.top_twenty_five().len(), 7);
        assert_eq!(seven.top_ten(), seven.top_twenty_five());
        let decoded = SelectionReceiptV3::from_bytes(&seven.to_bytes().unwrap()).unwrap();
        assert_eq!(decoded, seven);
    }

    #[test]
    fn only_recomputed_sidecar_admitted_status_enters_topn() {
        assert!(sidecar_status_is_admitted(AdmissionStatusV1::Admitted));
        assert!(!sidecar_status_is_admitted(AdmissionStatusV1::Rejected));
        assert!(!sidecar_status_is_admitted(AdmissionStatusV1::Unmeasured));
        assert!(!sidecar_status_is_admitted(AdmissionStatusV1::Refused));
    }

    #[test]
    fn two_pass_top_twenty_five_resolves_exact_rows_and_duplicate_alias_refuses() {
        let references = [
            reference(InstrumentFamilyV1::Nifty, 100, 18),
            reference(InstrumentFamilyV1::BankNifty, 110, 17),
        ];
        let candidates: Vec<_> = (0_u8..35)
            .map(|index| Candidate {
                strategy_digest: StrategyDigest::new(digest(index.wrapping_add(150))),
                mask_words: [1_u64 << u32::from(index % 63), 0, 0, 0, 0, 0],
                direction: if index.is_multiple_of(2) {
                    Direction::Long
                } else {
                    Direction::Short
                },
                admitted: !index.is_multiple_of(5),
                metrics: Metrics {
                    drawdown: u64::from(100_u8.saturating_sub(index)),
                    worst_loss: u64::from(90_u8.saturating_sub(index)),
                    losing_rate_ppm: u64::from(index) * 1_000,
                    losing_trades: u64::from(index),
                    loss_ratio_ppm: Some(u64::from(index) * 10_000),
                    pessimistic_profit: i64::from(index) * 100,
                    winning_trades: u64::from(index) + 1,
                    win_rate_ppm: 1_000_000_u64.saturating_sub(u64::from(index) * 1_000),
                    reward_to_risk_ppm: Some(1_000_000 + u64::from(index)),
                    average_win: 100 + u64::from(index),
                    average_loss: u64::from(35_u8.saturating_sub(index)),
                    assurance_ppm: 800_000 + u64::from(index),
                },
            })
            .collect();
        let mut extrema = Extrema::default();
        let mut pass = PopulationPass::new();
        for candidate in &candidates {
            extrema.observe(candidate);
            pass.observe(candidate);
        }
        let proof = pass.finish();
        let policy = RankingPolicyV1::new(Weights::equal()).expect("equal ranking policy");
        let mut keeper =
            VerifiedKeeper::new(MAX_TOP, policy, extrema, proof).expect("bounded verified keeper");
        for candidate in &candidates {
            keeper.offer(*candidate).expect("same second pass");
        }
        let selection = keeper.finish().expect("exact two-pass proof");
        assert_eq!(selection.rows.len(), 25);
        let mut resolved = vec![None; selection.rows.len()];
        for (index, candidate) in candidates.iter().enumerate() {
            let (source, row_sequence) = if index < 18 {
                (references[0], u64::try_from(index).expect("NIFTY sequence"))
            } else {
                (
                    references[1],
                    u64::try_from(index - 18).expect("BANKNIFTY sequence"),
                )
            };
            resolve_candidate(
                &mut resolved,
                &selection,
                source,
                row_sequence,
                candidate.strategy_digest.bytes(),
                *candidate,
            )
            .expect("resolve exact candidate");
        }
        assert!(resolved.iter().all(Option::is_some));

        let winner = selection.rows.first().expect("one ranked winner").candidate;
        let winner_index = candidates
            .iter()
            .position(|candidate| *candidate == winner)
            .expect("winner source row");
        let source = if winner_index < 18 {
            references[0]
        } else {
            references[1]
        };
        let duplicate = resolve_candidate(
            &mut resolved,
            &selection,
            source,
            u64::try_from(winner_index % 18).expect("winner sequence"),
            winner.strategy_digest.bytes(),
            winner,
        )
        .unwrap_err();
        assert!(duplicate.contains("occurs more than once"));
    }

    #[test]
    fn ledger_reopens_indexes_and_isolates_latest_by_exact_cohort_and_rung() {
        let root = root("latest");
        let first = receipt(6, 60, 4);
        let mut replacement = first.clone();
        replacement.populations[0].admission_completion_digest = digest(250);
        replacement.selection_id = replacement.derived_id().unwrap();
        replacement.validate_semantics().unwrap();
        let other_rung = receipt(6, 120, 4);
        let other_cohort = receipt(7, 60, 4);

        {
            let mut ledger = SelectionLedgerV3::open(&root, 8).expect("create V3 ledger");
            assert_eq!(ledger.selections(), 0);
            ledger.append(&first).expect("append first");
            ledger.append(&replacement).expect("append replacement");
            ledger.append(&other_rung).expect("append other rung");
            ledger.append(&other_cohort).expect("append other cohort");
            assert_eq!(ledger.selections(), 4);
            assert_eq!(
                ledger
                    .latest_selection(first.cohort_digest(), first.rung_seconds())
                    .unwrap()
                    .selection_id(),
                replacement.selection_id()
            );
            assert_eq!(
                ledger
                    .latest_selection(other_rung.cohort_digest(), 120)
                    .unwrap()
                    .selection_id(),
                other_rung.selection_id()
            );
            assert_eq!(
                ledger
                    .latest_selection(other_cohort.cohort_digest(), 60)
                    .unwrap()
                    .selection_id(),
                other_cohort.selection_id()
            );
            assert!(
                ledger
                    .append(&first)
                    .unwrap_err()
                    .contains("already present")
            );
        }

        let ledger = SelectionLedgerV3::open_read(&root, 4).expect("reopen V3 ledger");
        assert_eq!(ledger.selection_ids().len(), 4);
        assert_eq!(ledger.receipt(&first.selection_id()), Some(&first));
        assert!(SelectionLedgerV3::open_read(&root, 3).is_err());
        cleanup(&root);
    }

    #[test]
    fn header_is_exact_and_v1_or_v2_magic_never_falls_back() {
        let root = root("header");
        {
            let _ledger = SelectionLedgerV3::open(&root, 1).expect("create header");
        }
        let raw = std::fs::read(SelectionLedgerV3::path(&root)).expect("read V3 header");
        assert_eq!(raw.len(), HEADER_BYTES);
        assert_eq!(&raw[..8], &MAGIC_V3);
        assert_eq!(
            u32::from_le_bytes(raw[8..12].try_into().unwrap()),
            VERSION_V3
        );
        assert_eq!(
            u32::from_le_bytes(raw[16..20].try_into().unwrap()),
            STRIDE_BYTES_U32_V3
        );

        let mut wrong = raw;
        wrong[..8].copy_from_slice(b"BRUTXSL2");
        std::fs::write(SelectionLedgerV3::path(&root), wrong).expect("write wrong magic");
        assert!(
            SelectionLedgerV3::open_read(&root, 1)
                .unwrap_err()
                .contains("magic is unknown")
        );
        cleanup(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn same_length_external_mutation_makes_writable_handle_stale() {
        let root = root("stale");
        let first = receipt(8, 60, 2);
        let second = receipt(9, 60, 2);
        let mut ledger = SelectionLedgerV3::open(&root, 4).expect("create stale fixture");
        ledger.append(&first).expect("append first");

        let path = SelectionLedgerV3::path(&root);
        let mut external = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open external writer");
        external
            .seek(SeekFrom::Start(HEADER + 100))
            .expect("seek mutation");
        external.write_all(&[0x5a]).expect("same-length mutation");
        external.sync_all().expect("sync mutation");
        let refusal = ledger.append(&second).unwrap_err();
        assert!(
            refusal.contains("generation changed") || refusal.contains("changed while"),
            "unexpected stale refusal: {refusal}"
        );
        drop(ledger);
        cleanup(&root);
    }
}
