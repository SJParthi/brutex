//! Version-separated actual initial-empty and evaluated stored family authority.
use super::{
    AdmittedRootV1, BaseEvidenceLedgerBoundsV2, BaseEvidenceLedgerReaderV2,
    BaseEvidenceReopenAuditV2, CandidateUniverseBoundsV1, CandidateUniverseLedgerV1,
    CandidateUniverseReopenAuditV1, CommittedCandidatePreAdmissionV1,
    CommittedStoredCandidatePreAdmissionV1, PairedBaseEvidenceAuthorityV2,
    PairedBaseEvidenceReaderV2, PreAdmissionDataBoundsV1, PreAdmissionDataBoundsV2,
    PreAdmissionProductionCommitV2, ProducedPreAdmissionDataV2, RetainedStoredExecutionContextV1,
    StoredSearchMemberV4, pair_candidate_base_evidence_v2, reopened_execution_facts_v1,
    require_exact_stored_execution_join, require_reopened_pre_admission_context_v1, strict,
};
use std::path::Path;
use std::sync::Arc;

/// V1 exists only for evaluated rows. Empty runs carry their real V2 proof.
pub(crate) enum StoredFamilyV6 {
    Evaluated(Box<CommittedStoredCandidatePreAdmissionV1>),
    Extinct(Box<StoredExtinctFamilyV6>),
}

pub(crate) struct StoredExtinctFamilyV6 {
    pub(super) root: AdmittedRootV1,
    pub(super) candidate: CandidateUniverseReopenAuditV1,
    pub(super) candidate_bounds: CandidateUniverseBoundsV1,
    pub(super) base: BaseEvidenceReopenAuditV2,
    pub(super) base_bounds: BaseEvidenceLedgerBoundsV2,
    pub(super) pre_admission: ProducedPreAdmissionDataV2,
    pub(super) pre_admission_commit: PreAdmissionProductionCommitV2,
    pub(super) pre_admission_bounds: PreAdmissionDataBoundsV1,
    pub(super) search: StoredSearchMemberV4,
    pub(super) execution: RetainedStoredExecutionContextV1,
}

pub(super) enum CandidateCommitV6 {
    Evaluated(Box<CommittedCandidatePreAdmissionV1>),
    Extinct(Box<ExtinctCandidateV6>),
}

pub(super) struct ExtinctCandidateV6 {
    pub(super) candidate: CandidateUniverseReopenAuditV1,
    pub(super) candidate_bounds: CandidateUniverseBoundsV1,
    pub(super) base: BaseEvidenceReopenAuditV2,
    pub(super) base_bounds: BaseEvidenceLedgerBoundsV2,
    pub(super) pre_admission: ProducedPreAdmissionDataV2,
    pub(super) pre_admission_commit: PreAdmissionProductionCommitV2,
    pub(super) pre_admission_bounds: PreAdmissionDataBoundsV1,
}

impl CandidateCommitV6 {
    pub(super) const fn candidate_audit(&self) -> CandidateUniverseReopenAuditV1 {
        match self {
            Self::Evaluated(source) => source.candidate_audit(),
            Self::Extinct(source) => source.candidate,
        }
    }
}

impl From<CommittedStoredCandidatePreAdmissionV1> for StoredFamilyV6 {
    fn from(value: CommittedStoredCandidatePreAdmissionV1) -> Self {
        Self::Evaluated(Box::new(value))
    }
}

impl StoredFamilyV6 {
    pub(crate) fn candidate_audit(&self) -> CandidateUniverseReopenAuditV1 {
        match self {
            Self::Evaluated(source) => source.committed.candidate_audit(),
            Self::Extinct(source) => source.candidate,
        }
    }

    pub(crate) fn evaluated(&self) -> Option<&CommittedStoredCandidatePreAdmissionV1> {
        match self {
            Self::Evaluated(source) => Some(source),
            Self::Extinct(_) => None,
        }
    }

    pub(crate) fn pre_admission_v2(
        &self,
    ) -> (&ProducedPreAdmissionDataV2, &PreAdmissionProductionCommitV2) {
        match self {
            Self::Evaluated(source) => source.committed.pre_admission_v2(),
            Self::Extinct(source) => (&source.pre_admission, &source.pre_admission_commit),
        }
    }

    pub(crate) fn strict_inputs(&self) -> Option<Arc<strict::Inputs>> {
        match self {
            Self::Evaluated(source) => source.strict_inputs(),
            Self::Extinct(source) => source.execution.stored.strict.clone(),
        }
    }

    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.root()
            .require_same("while authenticating complete stored V6 family")?;
        match self {
            Self::Evaluated(source) => {
                source.open_candidate_reader()?;
            }
            Self::Extinct(source) => {
                let reader = CandidateUniverseLedgerV1::open_read(
                    source.root.path(),
                    source.candidate_bounds,
                )?;
                if reader.reopen_audit(&source.candidate.universe_id())? != Some(source.candidate) {
                    return Err("extinct V6 Candidate authority changed".to_owned());
                }
                require_v2(
                    source.root.path(),
                    source.pre_admission_bounds,
                    &source.pre_admission,
                    &source.pre_admission_commit,
                )?;
                require_base(source.root.path(), source.base_bounds, &source.base)?;
                source.search.projection()?;
            }
        }
        self.root()
            .require_same("after authenticating complete stored V6 family")
    }

    pub(super) fn root(&self) -> &AdmittedRootV1 {
        match self {
            Self::Evaluated(source) => &source.root,
            Self::Extinct(source) => &source.root,
        }
    }

    pub(super) fn search(&self) -> &StoredSearchMemberV4 {
        match self {
            Self::Evaluated(source) => &source.search,
            Self::Extinct(source) => &source.search,
        }
    }

    pub(super) fn base(&self) -> BaseEvidenceReopenAuditV2 {
        match self {
            Self::Evaluated(source) => *source.committed.base_evidence_audit(),
            Self::Extinct(source) => source.base,
        }
    }

    pub(super) fn base_bounds(&self) -> BaseEvidenceLedgerBoundsV2 {
        match self {
            Self::Evaluated(source) => source.committed.base_evidence_bounds(),
            Self::Extinct(source) => source.base_bounds,
        }
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        Option<CommittedStoredCandidatePreAdmissionV1>,
        Option<StoredExtinctFamilyV6>,
    ) {
        match self {
            Self::Evaluated(source) => (Some(*source), None),
            Self::Extinct(source) => (None, Some(*source)),
        }
    }
}

pub(super) fn require_evaluated_support(
    source: &CommittedStoredCandidatePreAdmissionV1,
) -> Result<(), String> {
    let expected = source.committed.pre_admission_audit();
    let ledger = crate::pre_admission_data::PreAdmissionDataLedgerV1::open_read(
        source.root.path(),
        source.committed.pre_admission_bounds,
    )?;
    if ledger.reopen_audit(&expected.authority_id())? != Some(expected) {
        return Err("evaluated V6 Pre-Admission V1 authority changed".to_owned());
    }
    let (produced, committed) = source.committed.pre_admission_v2();
    require_v2(
        source.root.path(),
        source.committed.pre_admission_bounds,
        produced,
        committed,
    )?;
    require_base(
        source.root.path(),
        source.committed.base_evidence_bounds(),
        source.committed.base_evidence_audit(),
    )?;
    source.search.projection()?;
    Ok(())
}

fn require_v2(
    root: &Path,
    bounds: PreAdmissionDataBoundsV1,
    produced: &ProducedPreAdmissionDataV2,
    committed: &PreAdmissionProductionCommitV2,
) -> Result<(), String> {
    let bounds = PreAdmissionDataBoundsV2::new(bounds.max_rows(), bounds.max_file_bytes())?;
    let expected = committed.audit();
    let ledger = crate::pre_admission_data::PreAdmissionDataLedgerV2::open_read(root, bounds)?;
    if ledger.reopen_audit(&expected.authority_id())? != Some(expected) {
        return Err("V6 Pre-Admission V2 authority changed".to_owned());
    }
    produced.authenticated_observation_source(committed)?;
    Ok(())
}

fn require_base(
    root: &Path,
    bounds: BaseEvidenceLedgerBoundsV2,
    expected: &BaseEvidenceReopenAuditV2,
) -> Result<(), String> {
    let mut reader =
        BaseEvidenceLedgerReaderV2::open(root, bounds).map_err(|why| why.to_string())?;
    if reader
        .audit(expected.candidate_universe_id())
        .map_err(|why| why.to_string())?
        != Some(*expected)
    {
        return Err("V6 Base evidence changed".to_owned());
    }
    Ok(())
}

pub(super) fn reopen_pair(
    nifty: &StoredFamilyV6,
    banknifty: &StoredFamilyV6,
) -> Result<(PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2), String> {
    nifty.require_current()?;
    banknifty.require_current()?;
    super::require_exact_admitted_source_root_pair_v2(
        nifty.root(),
        banknifty.root(),
        "before strict V6 Base pair",
    )?;
    let mut reader = BaseEvidenceLedgerReaderV2::open(nifty.root().path(), nifty.base_bounds())
        .map_err(|why| why.to_string())?;
    let nifty_expected = nifty.base();
    let bank_expected = banknifty.base();
    let first = reader
        .audit(nifty_expected.candidate_universe_id())
        .map_err(|why| why.to_string())?
        .ok_or("V6 NIFTY Base completion missing")?;
    let second = reader
        .audit(bank_expected.candidate_universe_id())
        .map_err(|why| why.to_string())?
        .ok_or("V6 BANKNIFTY Base completion missing")?;
    if first != nifty_expected || second != bank_expected {
        return Err("V6 Base pair changed".to_owned());
    }
    let pair = pair_candidate_base_evidence_v2(&first, &second)?;
    let reader = reader.bind_pair(&pair).map_err(|why| why.to_string())?;
    nifty.require_current()?;
    banknifty.require_current()?;
    Ok((pair, reader))
}

pub(super) fn finish(
    root: AdmittedRootV1,
    committed: CandidateCommitV6,
    search: StoredSearchMemberV4,
    execution: RetainedStoredExecutionContextV1,
) -> Result<StoredFamilyV6, String> {
    root.require_same("before final retained/reopened authority join")?;
    let committed = match committed {
        CandidateCommitV6::Evaluated(committed) => *committed,
        CandidateCommitV6::Extinct(source) => {
            let ExtinctCandidateV6 {
                candidate,
                candidate_bounds,
                base,
                base_bounds,
                pre_admission,
                pre_admission_commit,
                pre_admission_bounds,
            } = *source;
            let family = StoredFamilyV6::Extinct(Box::new(StoredExtinctFamilyV6 {
                root,
                candidate,
                candidate_bounds,
                base,
                base_bounds,
                pre_admission,
                pre_admission_commit,
                pre_admission_bounds,
                search,
                execution,
            }));
            family.require_current()?;
            return Ok(family);
        }
    };
    let runtime_facts = execution.stored_facts(&committed)?;
    let reopened_facts = reopened_execution_facts_v1(&committed)?;
    require_exact_stored_execution_join(&runtime_facts, &reopened_facts)?;
    require_reopened_pre_admission_context_v1(&committed, &runtime_facts)?;
    let reopened_signal_bars = usize::try_from(reopened_facts.signal.count)
        .map_err(|_| "Step 3 reopened signal count does not fit usize".to_owned())?;
    if execution.search_splits != crate::walk_forward_splits(reopened_signal_bars)
        || execution.search_splits < 2
    {
        return Err(
            "Step 3 retained Search V4 split count differs from the canonical reopened signal policy"
                .to_owned(),
        );
    }
    Ok(CommittedStoredCandidatePreAdmissionV1 {
        root,
        committed,
        search,
        execution,
    }
    .into())
}
